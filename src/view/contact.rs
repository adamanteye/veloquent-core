use super::*;
use entity::{
    contact,
    prelude::{Contact, Session, User},
    session, user,
};

/// 发起添加好友
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact/new",
    params(
        ("id" = Uuid, Path, description = "要添加的用户主键") ), responses(
        (status = 200, description = "发起成功")
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn add_contact_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(contact): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let con: Option<user::Model> = User::find_by_id(contact).one(&state.conn).await?;
    let con = con.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        contact
    )))?;
    if user.id == con.id {
        return Err(AppError::BadRequest("cannot add self".to_string()));
    }
    let l = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::User.eq(user.id))
                .add(contact::Column::RefUser.eq(con.id)),
        )
        .one(&state.conn)
        .await?;
    if l.is_some() {
        return Err(AppError::Conflict(format!(
            "contact relation exist [{}:{}]",
            user.id, con.id
        )));
    }
    let l = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::User.eq(con.id))
                .add(contact::Column::RefUser.eq(user.id)),
        )
        .one(&state.conn)
        .await?;
    if l.is_some() {
        return Err(AppError::Conflict(format!(
            "contact relation exist [{}:{}]",
            con.id, user.id
        )));
    }
    let s = session::ActiveModel {
        id: ActiveValue::not_set(),
        created_at: ActiveValue::not_set(),
        name: ActiveValue::not_set(),
    };
    let s = Session::insert(s).exec(&state.conn).await?.last_insert_id;
    let c = contact::ActiveModel {
        id: ActiveValue::not_set(),
        user: ActiveValue::set(user.id),
        ref_user: ActiveValue::set(Some(con.id)),
        alias: ActiveValue::set(con.alias),
        chat: ActiveValue::set(s),
        created_at: ActiveValue::not_set(),
        category: ActiveValue::not_set(),
    };
    Contact::insert(c).exec(&state.conn).await?;
    Ok(StatusCode::OK.into_response())
}

/// 删除好友
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact/delete",
    params(
        ("id" = Uuid, Path, description = "要删除的用户主键") ), responses(
        (status = 204, description = "删除成功")
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn delete_contact_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(con): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut c: contact::ActiveModel = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::User.eq(payload.id))
                .add(contact::Column::RefUser.eq(con)),
        )
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!(
            "cannot find contact [{}] of [{}]",
            con, payload.id
        )))?
        .into();
    let mut u: contact::ActiveModel = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::RefUser.eq(payload.id))
                .add(contact::Column::User.eq(con)),
        )
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!(
            "cannot find contact [{}] of [{}]",
            payload.id, con
        )))?
        .into();
    c.ref_user = ActiveValue::set(None);
    u.ref_user = ActiveValue::set(None);
    Contact::update(c).exec(&state.conn).await?;
    Contact::update(u).exec(&state.conn).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// 接受添加好友
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact/accept",
    params(
        ("id" = Uuid, Path, description = "要接受的用户主键") ), responses(
        (status = 200, description = "接受成功")
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn accept_contact_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(contact): Path<Uuid>,
) -> Result<Response, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let con: Option<user::Model> = User::find_by_id(contact).one(&state.conn).await?;
    let con = con.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        contact
    )))?;
    if user.id == con.id {
        return Err(AppError::BadRequest("cannot accept self".to_string()));
    }
    let l = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::User.eq(user.id))
                .add(contact::Column::RefUser.eq(con.id)),
        )
        .one(&state.conn)
        .await?;
    if l.is_some() {
        return Err(AppError::Conflict(format!(
            "contact already accepted [{}:{}]",
            user.id, con.id
        )));
    }
    let entry = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::User.eq(con.id))
                .add(contact::Column::RefUser.eq(user.id)),
        )
        .one(&state.conn)
        .await?
        .ok_or(anyhow::anyhow!(
            "contact not found [{}:{}]",
            con.id,
            user.id
        ))?;
    let s = Session::find_by_id(entry.chat)
        .one(&state.conn)
        .await?
        .ok_or(anyhow::anyhow!("session not found [{}]", entry.chat))?;
    let c = contact::ActiveModel {
        id: ActiveValue::not_set(),
        user: ActiveValue::set(user.id),
        ref_user: ActiveValue::set(Some(con.id)),
        alias: ActiveValue::set(con.alias),
        chat: ActiveValue::set(s.id),
        created_at: ActiveValue::not_set(),
        category: ActiveValue::not_set(),
    };
    Contact::insert(c).exec(&state.conn).await?;
    Ok(StatusCode::OK.into_response())
}

/// 好友(申请)列表
///
/// 用于返回好友列表与返回好友申请
#[derive(prost::Message)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct ContactList {
    /// 好友(申请)数量
    ///
    /// `tag` = `1`
    #[prost(int32, tag = "1")]
    pub num: i32,
    /// 好友(申请者)的UUID
    ///
    /// `tag` = `2`
    #[prost(message, repeated, tag = "2")]
    pub user: Vec<Chat>,
}

#[derive(prost::Message)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct Chat {
    /// 好友(申请者)的UUID
    ///
    /// `tag` = `1`
    #[prost(string, tag = "1")]
    pub user: String,
    /// 会话
    ///
    /// `tag` = `2`
    #[prost(string, tag = "2")]
    pub session: String,
}

#[derive(Debug, FromQueryResult)]
struct UserUuid {
    user: Uuid,
    chat: Uuid,
}

impl ContactList {
    async fn query_contact(user: user::Model, db: &DatabaseConnection) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
                    "SELECT a.user, a.chat FROM contact AS a INNER JOIN contact AS b ON a.user = b.ref_user AND a.ref_user = b.user WHERE a.user = $1",[user.id.into()])).all(db).await?;
        let user = contacts
            .iter()
            .map(|c| Chat {
                user: c.user.to_string(),
                session: c.chat.to_string(),
            })
            .collect();
        let num = contacts.len() as i32;
        Ok(Self { num, user })
    }

    async fn query_new_contact(
        user: user::Model,
        db: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
            "SELECT contact.user, contact.chat FROM contact WHERE contact.ref_user = $1 EXCEPT SELECT contact.ref_user FROM contact WHERE contact.user = $1",[user.id.into()])).all(db).await?;
        let num = contacts.len() as i32;
        let user = contacts
            .iter()
            .map(|c| Chat {
                user: c.user.to_string(),
                session: c.chat.to_string(),
            })
            .collect();
        Ok(Self { num, user })
    }

    async fn query_pending_contact(
        user: user::Model,
        db: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
                    "SELECT contact.ref_user, contact.chat FROM contact WHERE contact.user = $1 EXCEPT SELECT contact.user FROM contact WHERE contact.ref_user = $1",[user.id.into()])).all(db).await?;
        let num = contacts.len() as i32;
        let user = contacts
            .iter()
            .map(|c| Chat {
                user: c.user.to_string(),
                session: c.chat.to_string(),
            })
            .collect();
        Ok(Self { num, user })
    }
}

/// 推送好友申请列表
///
/// 返回 Protobuf 格式数据
#[instrument(skip(state))]
#[cfg_attr(feature = "dev", utoipa::path(post, path = "/ws", tag = "contact"))]
pub async fn get_new_contacts_handler(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user: Option<user::Model> = User::find_by_id(user_id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        user_id
    )))?;
    event!(Level::INFO, "get new contact list of user [{}]", user_id);
    Ok(Protobuf(
        ContactList::query_new_contact(user, &state.conn).await?,
    ))
}

/// 获取好友列表
///
/// 返回 Protobuf 格式数据
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact/list",
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn get_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Protobuf<ContactList>, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    event!(Level::INFO, "get contact list of user [{}]", payload.id);
    Ok(Protobuf(
        ContactList::query_contact(user, &state.conn).await?,
    ))
}

/// 获取待通过好友列表
///
/// 返回 Protobuf 格式数据
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact/pending",
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
pub async fn get_pending_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Protobuf<ContactList>, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    event!(
        Level::INFO,
        "get pending contact list of user [{}]",
        payload.id
    );
    Ok(Protobuf(
        ContactList::query_pending_contact(user, &state.conn).await?,
    ))
}
