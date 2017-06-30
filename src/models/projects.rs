use chrono::NaiveDateTime;
use schema::projects;
use super::users::User;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct Project {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[table_name = "projects"]
pub struct NewProject<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub description: Option<&'a str>,
}
