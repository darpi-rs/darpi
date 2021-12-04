use async_graphql::connection::{query, Connection, Edge, EmptyFields};
use async_graphql::{Context, Enum, Interface, Object};
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use darpi::{
    app, handler, logger::DefaultFormat, shaku::module, shaku::Component, shaku::Interface, App,
    Path,
};
use darpi_graphql::{GraphQLBody, MultipartOptionsProviderImpl, Request, Response};
use darpi_middleware::{body_size_limit, log_request, log_response};
use env_logger;
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::collections::HashMap;
use std::sync::Arc;

/// One of the films in the Star Wars Trilogy
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum Episode {
    /// Released in 1977.
    NewHope,

    /// Released in 1980.
    Empire,

    /// Released in 1983.
    Jedi,
}

pub struct Human(usize);

/// A humanoid creature in the Star Wars universe.
#[Object]
impl Human {
    /// The id of the human.
    async fn id(&self, ctx: &Context<'_>) -> &str {
        ctx.data_unchecked::<StarWars>().chars[self.0].id
    }

    /// The name of the human.
    async fn name(&self, ctx: &Context<'_>) -> &str {
        ctx.data_unchecked::<StarWars>().chars[self.0].name
    }

    /// The friends of the human, or an empty list if they have none.
    async fn friends(&self, ctx: &Context<'_>) -> Vec<Character> {
        ctx.data_unchecked::<StarWars>().chars[self.0]
            .friends
            .iter()
            .map(|id| Human(*id).into())
            .collect()
    }

    /// Which movies they appear in.
    async fn appears_in<'a>(&self, ctx: &'a Context<'_>) -> &'a [Episode] {
        &ctx.data_unchecked::<StarWars>().chars[self.0].appears_in
    }

    /// The home planet of the human, or null if unknown.
    async fn home_planet<'a>(&self, ctx: &'a Context<'_>) -> &'a Option<&'a str> {
        &ctx.data_unchecked::<StarWars>().chars[self.0].home_planet
    }
}

pub struct Droid(usize);

/// A mechanical creature in the Star Wars universe.
#[Object]
impl Droid {
    /// The id of the droid.
    async fn id(&self, ctx: &Context<'_>) -> &str {
        ctx.data_unchecked::<StarWars>().chars[self.0].id
    }

    /// The name of the droid.
    async fn name(&self, ctx: &Context<'_>) -> &str {
        ctx.data_unchecked::<StarWars>().chars[self.0].name
    }

    /// The friends of the droid, or an empty list if they have none.
    async fn friends(&self, ctx: &Context<'_>) -> Vec<Character> {
        ctx.data_unchecked::<StarWars>().chars[self.0]
            .friends
            .iter()
            .map(|id| Droid(*id).into())
            .collect()
    }

    /// Which movies they appear in.
    async fn appears_in<'a>(&self, ctx: &'a Context<'_>) -> &'a [Episode] {
        &ctx.data_unchecked::<StarWars>().chars[self.0].appears_in
    }

    /// The primary function of the droid.
    async fn primary_function<'a>(&self, ctx: &'a Context<'_>) -> &'a Option<&'a str> {
        &ctx.data_unchecked::<StarWars>().chars[self.0].primary_function
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hero(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "If omitted, returns the hero of the whole saga. If provided, returns the hero of that particular episode."
        )]
        episode: Episode,
    ) -> Character {
        if episode == Episode::Empire {
            Human(ctx.data_unchecked::<StarWars>().luke).into()
        } else {
            Droid(ctx.data_unchecked::<StarWars>().artoo).into()
        }
    }

    async fn human(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "id of the human")] id: String,
    ) -> Option<Human> {
        ctx.data_unchecked::<StarWars>().human(&id).map(Human)
    }

    async fn humans(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> async_graphql::Result<Connection<usize, Human, EmptyFields, EmptyFields>> {
        let humans = ctx
            .data_unchecked::<StarWars>()
            .humans()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        query_characters(after, before, first, last, &humans)
            .await
            .map(|conn| conn.map_node(Human))
    }

    async fn droid(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "id of the droid")] id: String,
    ) -> Option<Droid> {
        ctx.data_unchecked::<StarWars>().droid(&id).map(Droid)
    }

    async fn droids(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> async_graphql::Result<Connection<usize, Droid, EmptyFields, EmptyFields>> {
        let droids = ctx
            .data_unchecked::<StarWars>()
            .droids()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        query_characters(after, before, first, last, &droids)
            .await
            .map(|conn| conn.map_node(Droid))
    }
}

#[derive(Interface)]
#[graphql(
    field(name = "id", type = "&str"),
    field(name = "name", type = "&str"),
    field(name = "friends", type = "Vec<Character>"),
    field(name = "appears_in", type = "&'ctx [Episode]")
)]
pub enum Character {
    Human(Human),
    Droid(Droid),
}

async fn query_characters(
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    characters: &[usize],
) -> async_graphql::Result<Connection<usize, usize, EmptyFields, EmptyFields>> {
    query(
        after,
        before,
        first,
        last,
        |after, before, first, last| async move {
            let mut start = 0usize;
            let mut end = characters.len();

            if let Some(after) = after {
                if after >= characters.len() {
                    return Ok(Connection::new(false, false));
                }
                start = after + 1;
            }

            if let Some(before) = before {
                if before == 0 {
                    return Ok(Connection::new(false, false));
                }
                end = before;
            }

            let mut slice = &characters[start..end];

            if let Some(first) = first {
                slice = &slice[..first.min(slice.len())];
                end -= first.min(slice.len());
            } else if let Some(last) = last {
                slice = &slice[slice.len() - last.min(slice.len())..];
                start = end - last.min(slice.len());
            }

            let mut connection = Connection::new(start > 0, end < characters.len());
            connection.append(
                slice
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| Edge::new(start + idx, *item)),
            );
            Ok(connection)
        },
    )
    .await
}

pub type StarWarsSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub struct StarWarsChar {
    id: &'static str,
    name: &'static str,
    friends: Vec<usize>,
    appears_in: Vec<Episode>,
    home_planet: Option<&'static str>,
    primary_function: Option<&'static str>,
}

pub struct StarWars {
    luke: usize,
    artoo: usize,
    chars: Slab<StarWarsChar>,
    human_data: HashMap<&'static str, usize>,
    droid_data: HashMap<&'static str, usize>,
}

impl StarWars {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut chars = Slab::new();

        let luke = chars.insert(StarWarsChar {
            id: "1000",
            name: "Luke Skywalker",
            friends: vec![],
            appears_in: vec![],
            home_planet: Some("Tatooine"),
            primary_function: None,
        });

        let vader = chars.insert(StarWarsChar {
            id: "1001",
            name: "Luke Skywalker",
            friends: vec![],
            appears_in: vec![],
            home_planet: Some("Tatooine"),
            primary_function: None,
        });

        let han = chars.insert(StarWarsChar {
            id: "1002",
            name: "Han Solo",
            friends: vec![],
            appears_in: vec![Episode::Empire, Episode::NewHope, Episode::Jedi],
            home_planet: None,
            primary_function: None,
        });

        let leia = chars.insert(StarWarsChar {
            id: "1003",
            name: "Leia Organa",
            friends: vec![],
            appears_in: vec![Episode::Empire, Episode::NewHope, Episode::Jedi],
            home_planet: Some("Alderaa"),
            primary_function: None,
        });

        let tarkin = chars.insert(StarWarsChar {
            id: "1004",
            name: "Wilhuff Tarkin",
            friends: vec![],
            appears_in: vec![Episode::Empire, Episode::NewHope, Episode::Jedi],
            home_planet: None,
            primary_function: None,
        });

        let threepio = chars.insert(StarWarsChar {
            id: "2000",
            name: "C-3PO",
            friends: vec![],
            appears_in: vec![Episode::Empire, Episode::NewHope, Episode::Jedi],
            home_planet: None,
            primary_function: Some("Protocol"),
        });

        let artoo = chars.insert(StarWarsChar {
            id: "2001",
            name: "R2-D2",
            friends: vec![],
            appears_in: vec![Episode::Empire, Episode::NewHope, Episode::Jedi],
            home_planet: None,
            primary_function: Some("Astromech"),
        });

        chars[luke].friends = vec![han, leia, threepio, artoo];
        chars[vader].friends = vec![tarkin];
        chars[han].friends = vec![luke, leia, artoo];
        chars[leia].friends = vec![luke, han, threepio, artoo];
        chars[tarkin].friends = vec![vader];
        chars[threepio].friends = vec![luke, han, leia, artoo];
        chars[artoo].friends = vec![luke, han, leia];

        let mut human_data = HashMap::new();
        human_data.insert("1000", luke);
        human_data.insert("1001", vader);
        human_data.insert("1002", han);
        human_data.insert("1003", leia);
        human_data.insert("1004", tarkin);

        let mut droid_data = HashMap::new();
        droid_data.insert("2000", threepio);
        droid_data.insert("2001", artoo);

        Self {
            luke,
            artoo,
            chars,
            human_data,
            droid_data,
        }
    }

    pub fn human(&self, id: &str) -> Option<usize> {
        self.human_data.get(id).cloned()
    }

    pub fn droid(&self, id: &str) -> Option<usize> {
        self.droid_data.get(id).cloned()
    }

    pub fn humans(&self) -> Vec<usize> {
        self.human_data.values().cloned().collect()
    }

    pub fn droids(&self) -> Vec<usize> {
        self.droid_data.values().cloned().collect()
    }
}

fn make_container() -> Container {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(StarWars::new())
        .finish();

    let module = Container::builder()
        .with_component_parameters::<SchemaGetterImpl>(SchemaGetterImplParameters { schema })
        .build();
    module
}

trait SchemaGetter: Interface {
    fn get(&self) -> &StarWarsSchema;
}

#[derive(Component)]
#[shaku(interface = SchemaGetter)]
struct SchemaGetterImpl {
    #[shaku(default = unimplemented!())]
    schema: StarWarsSchema,
}

impl SchemaGetter for SchemaGetterImpl {
    fn get(&self) -> &StarWarsSchema {
        &self.schema
    }
}

module! {
    Container {
        components = [SchemaGetterImpl, MultipartOptionsProviderImpl],
        providers = [],
    }
}

#[handler({
    container: Container
})]
async fn index_get(
    #[inject] schema: Arc<dyn SchemaGetter>,
    #[query] req: GraphQLBody<Request>,
) -> Response {
    schema.get().execute(req.0.into_inner()).await.into()
}

#[handler({
    container: Container
})]
async fn index_post(
    #[inject] schema: Arc<dyn SchemaGetter>,
    #[body] req: GraphQLBody<Request>,
) -> Response {
    schema.get().execute(req.0.into_inner()).await.into()
}

#[handler({
    container: Container,
    middleware: {
        request: [],
        response: []
    },
    jobs: {
        request: [],
        response: []
    }
})]
pub(crate) async fn home() -> String {
    format!("home")
}

#[derive(Deserialize, Serialize, Debug, Path)]
pub struct Name {
    name: String,
}

#[handler({
    container: Container,
    middleware: {
        request: [body_size_limit(64)]
    }
})]
pub(crate) async fn do_something(#[path] p: Name) -> String {
    format!("user {}", p.name)
}

#[handler({
    container: Container,
    middleware: {
        request: [body_size_limit(64)]
    }
})]
pub(crate) async fn do_something1(#[query] p: Option<Name>) -> String {
    format!("user {:#?}", p)
}

#[darpi::main]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).try_init().unwrap();

    app!({
        address: "127.0.0.1:3000",
        container: {
            factory: make_container(),
            type: Container
        },
        jobs: {
            request: [],
            response: []
        },
        middleware: {
            request: [log_request(DefaultFormat)],
            response: [log_response(DefaultFormat, request(0))]
        },
        handlers: [{
            route: "/",
            method: GET,
            handler: index_get
        },{
            route: "/",
            method: POST,
            handler: index_post
        }]
    })
    .run()
    .await
}
