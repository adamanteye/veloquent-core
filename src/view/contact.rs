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
    };
    let s = Session::insert(s).exec(&state.conn).await?.last_insert_id;
    let c = contact::ActiveModel {
        id: ActiveValue::not_set(),
        user: ActiveValue::set(user.id),
        ref_user: ActiveValue::set(Some(con.id)),
        alias: ActiveValue::set(con.alias),
        session: ActiveValue::set(s),
        created_at: ActiveValue::not_set(),
        category: ActiveValue::not_set(),
    };
    Contact::insert(c).exec(&state.conn).await?;
    event!(Level::DEBUG, "create new session [{}]", s);
    event!(Level::DEBUG, "user [{}] add [{}]", user.id, con.id);
    let s = state.clone();
    tokio::task::spawn(async move {
        state
            .ws_pool
            .notify(con.id, notify_new_contacts(s, con.id).await)
    });
    Ok(StatusCode::OK.into_response())
}

/// 拒绝好友申请
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact/delete",
    params(
        ("id" = Uuid, Path, description = "要拒绝的用户主键") ), responses(
        (status = 200, description = "拒绝成功")
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn reject_contact_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(con): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let c: contact::ActiveModel = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::User.eq(con))
                .add(contact::Column::RefUser.eq(payload.id)),
        )
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!(
            "cannot find contact [{}] of [{}]",
            payload.id, con
        )))?
        .into();
    let u: Option<contact::Model> = Contact::find()
        .filter(
            Condition::all()
                .add(contact::Column::RefUser.eq(con))
                .add(contact::Column::User.eq(payload.id)),
        )
        .one(&state.conn)
        .await?;
    if u.is_some() {
        return Err(AppError::Conflict(format!(
            "contact relation exist [{}:{}]",
            payload.id, con
        )));
    }
    Contact::delete(c).exec(&state.conn).await?;
    Ok(StatusCode::OK.into_response())
}

/// 删除好友
#[cfg_attr(feature = "dev",
utoipa::path(delete, path = "/contact/delete",
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
    let s = Session::find_by_id(entry.session)
        .one(&state.conn)
        .await?
        .ok_or(anyhow::anyhow!("session not found [{}]", entry.session))?;
    let c = contact::ActiveModel {
        id: ActiveValue::not_set(),
        user: ActiveValue::set(user.id),
        ref_user: ActiveValue::set(Some(con.id)),
        alias: ActiveValue::set(con.alias),
        session: ActiveValue::set(s.id),
        created_at: ActiveValue::not_set(),
        category: ActiveValue::not_set(),
    };
    Contact::insert(c).exec(&state.conn).await?;
    Ok(StatusCode::OK.into_response())
}

/// 好友(申请)列表
///
/// 用于返回好友列表与返回好友申请
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct ContactList {
    /// 好友(申请)数量
    pub num: i32,
    /// 好友(申请者)的 UUID
    pub items: Vec<Chat>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct Chat {
    /// 好友(申请者)的 UUID
    ///
    /// 在通知中, 也可以表示群聊的 UUID
    pub id: Uuid,
    /// 会话
    pub session: Uuid,
}

#[derive(Debug, FromQueryResult)]
struct UserUuid {
    user: Uuid,
    session: Uuid,
}

impl ContactList {
    async fn query_contact(user: user::Model, db: &DatabaseConnection) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
                    "SELECT a.ref_user AS user, a.session FROM contact AS a INNER JOIN contact AS b ON a.user = b.ref_user AND a.ref_user = b.user WHERE a.user = $1",[user.id.into()])).all(db).await?;
        let items = contacts
            .iter()
            .map(|c| Chat {
                id: c.user,
                session: c.session,
            })
            .collect();
        let num = contacts.len() as i32;
        Ok(Self { num, items })
    }

    async fn query_new_contact(
        user: user::Model,
        db: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
            "SELECT contact.user, contact.session FROM contact WHERE contact.ref_user = $1 EXCEPT SELECT contact.ref_user, contact.session FROM contact WHERE contact.user = $1",[user.id.into()])).all(db).await?;
        let num = contacts.len() as i32;
        let items = contacts
            .iter()
            .map(|c| Chat {
                id: c.user,
                session: c.session,
            })
            .collect();
        Ok(Self { num, items })
    }

    async fn query_pending_contact(
        user: user::Model,
        db: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
                    "SELECT contact.ref_user AS user, contact.session FROM contact WHERE contact.user = $1 EXCEPT SELECT contact.user, contact.session FROM contact WHERE contact.ref_user = $1",[user.id.into()])).all(db).await?;
        let num = contacts.len() as i32;
        let items = contacts
            .iter()
            .map(|c| Chat {
                id: c.user,
                session: c.session,
            })
            .collect();
        Ok(Self { num, items })
    }
}

/// 推送好友申请列表
#[instrument(skip(state))]
pub async fn notify_new_contacts(
    state: AppState,
    user_id: Uuid,
) -> Result<WebSocketMessage, AppError> {
    let user: Option<user::Model> = User::find_by_id(user_id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        user_id
    )))?;
    let data = ContactList::query_new_contact(user, &state.conn).await?;
    event!(Level::DEBUG, "get new contact list of user [{}]", user_id);
    let data = Json(data);
    Ok(WebSocketMessage::Text(format!("{:?}", Json(data))))
}

/// 获取待通过好友列表
///
/// 即希望添加当前用户作为好友的用户列表
#[cfg_attr(feature = "dev",
utoipa::path(get, path = "/contact/new",
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn get_new_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<ContactList>, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let data = ContactList::query_new_contact(user, &state.conn).await?;
    event!(
        Level::DEBUG,
        "get new contact list [{:?}] of user [{}]",
        data,
        payload.id
    );
    Ok(Json(data))
}

/// 获取好友列表
#[cfg_attr(feature = "dev",
utoipa::path(get, path = "/contact/list",
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn get_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<ContactList>, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let data = ContactList::query_contact(user, &state.conn).await?;
    event!(
        Level::DEBUG,
        "get contact list [{:?}] of user [{}]",
        data,
        payload.id
    );
    Ok(Json(data))
}

/// 获取发起申请但待通过的好友列表
#[cfg_attr(feature = "dev",
utoipa::path(get, path = "/contact/pending",
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
pub async fn get_pending_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<ContactList>, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let data = ContactList::query_pending_contact(user, &state.conn).await?;
    event!(
        Level::DEBUG,
        "get pending contact list [{:?}] of user [{}]",
        data,
        payload.id
    );
    Ok(Json(data))
}
