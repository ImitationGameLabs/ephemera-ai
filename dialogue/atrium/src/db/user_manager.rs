use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, NotSet, QueryOrder};
use thiserror::Error;
use time::OffsetDateTime;

use crate::entity::{UserEntity};
use crate::entity::user;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sea_orm::DbErr),

    #[error("User not found with name: {0}")]
    UserNotFound(String),

    #[error("Invalid password for user: {0}")]
    InvalidPassword(String),

    #[error("User already exists with name: {0}")]
    UserAlreadyExists(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct UserDto {
    pub id: i32,
    pub name: String,
    pub bio: String,
    pub message_height: i32,
    pub last_seen: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub online: bool,
}

impl From<user::Model> for UserDto {
    fn from(model: user::Model) -> Self {
        let now = OffsetDateTime::now_utc();
        let online = model.last_seen
            .map(|last_seen| {
                let duration = now - last_seen;
                duration.whole_seconds() < 300 // 5 minutes timeout
            })
            .unwrap_or(false);

        Self {
            id: model.id,
            name: model.name,
            bio: model.bio,
            message_height: model.message_height,
            last_seen: model.last_seen,
            created_at: model.created_at,
            online,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateUserDto {
    pub name: String,
    pub bio: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct UpdateUserDto {
    pub bio: Option<String>,
    pub new_password: Option<String>,
}

#[derive(Clone)]
pub struct UserManager {
    conn: DatabaseConnection,
}

impl UserManager {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    pub async fn create_user(&self, user_dto: &CreateUserDto) -> Result<UserDto, UserError> {
        // Check if user already exists
        let existing_user = UserEntity::find()
            .filter(user::Column::Name.eq(&user_dto.name))
            .one(&self.conn)
            .await?;

        if existing_user.is_some() {
            return Err(UserError::UserAlreadyExists(user_dto.name.clone()));
        }

        let now = OffsetDateTime::now_utc();
        let active_model = user::ActiveModel {
            id: NotSet,
            name: Set(user_dto.name.clone()),
            bio: Set(user_dto.bio.clone()),
            password: Set(user_dto.password.clone()),
            message_height: Set(0),
            last_seen: Set(None),
            created_at: Set(now),
        };

        let inserted_model = active_model.insert(&self.conn).await?;
        Ok(inserted_model.into())
    }

    pub async fn get_user_by_name(&self, name: &str) -> Result<UserDto, UserError> {
        let model = UserEntity::find()
            .filter(user::Column::Name.eq(name))
            .one(&self.conn)
            .await?
            .ok_or(UserError::UserNotFound(name.to_string()))?;

        Ok(model.into())
    }

    pub async fn get_user_by_id(&self, id: i32) -> Result<UserDto, UserError> {
        let model = UserEntity::find_by_id(id)
            .one(&self.conn)
            .await?
            .ok_or(UserError::UserNotFound(format!("id: {}", id)))?;

        Ok(model.into())
    }

    pub async fn authenticate_user(&self, name: &str, password: &str) -> Result<UserDto, UserError> {
        let model = UserEntity::find()
            .filter(user::Column::Name.eq(name))
            .filter(user::Column::Password.eq(password))
            .one(&self.conn)
            .await?
            .ok_or(UserError::InvalidPassword(name.to_string()))?;

        Ok(model.into())
    }

    pub async fn authenticate_by_credentials(&self, username: &str, password: &str) -> Result<UserDto, UserError> {
        self.authenticate_user(username, password).await
    }

    pub async fn update_user(&self, name: &str, update_dto: &UpdateUserDto) -> Result<UserDto, UserError> {
        let model = UserEntity::find()
            .filter(user::Column::Name.eq(name))
            .one(&self.conn)
            .await?
            .ok_or(UserError::UserNotFound(name.to_string()))?;

        let mut active_model: user::ActiveModel = model.into();

        if let Some(bio) = &update_dto.bio {
            active_model.bio = Set(bio.clone());
        }

        if let Some(new_password) = &update_dto.new_password {
            active_model.password = Set(new_password.clone());
        }

        let updated_model = active_model.update(&self.conn).await?;
        Ok(updated_model.into())
    }

    pub async fn update_heartbeat(&self, name: &str) -> Result<UserDto, UserError> {
        let model = UserEntity::find()
            .filter(user::Column::Name.eq(name))
            .one(&self.conn)
            .await?
            .ok_or(UserError::UserNotFound(name.to_string()))?;

        let now = OffsetDateTime::now_utc();
        let mut active_model: user::ActiveModel = model.into();
        active_model.last_seen = Set(Some(now));

        let updated_model = active_model.update(&self.conn).await?;
        Ok(updated_model.into())
    }

    pub async fn update_message_height(&self, name: &str, message_height: i32) -> Result<UserDto, UserError> {
        let model = UserEntity::find()
            .filter(user::Column::Name.eq(name))
            .one(&self.conn)
            .await?
            .ok_or(UserError::UserNotFound(name.to_string()))?;

        let mut active_model: user::ActiveModel = model.into();
        active_model.message_height = Set(message_height);

        let updated_model = active_model.update(&self.conn).await?;
        Ok(updated_model.into())
    }

    pub async fn get_all_users(&self) -> Result<Vec<UserDto>, UserError> {
        let models = UserEntity::find()
            .order_by_asc(user::Column::Name)
            .all(&self.conn)
            .await?;

        Ok(models.into_iter().map(Into::into).collect())
    }
}