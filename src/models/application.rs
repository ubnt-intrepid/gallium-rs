use schema::applications;
use chrono::NaiveDateTime;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
pub struct Application {
    pub id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub client_id: String,
}

#[derive(Insertable)]
#[table_name = "applications"]
pub struct NewApplication<'a> {
    pub name: &'a str,
    pub client_id: &'a str,
}
