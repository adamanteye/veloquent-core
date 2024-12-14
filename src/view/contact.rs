use super::*;
use entity::{
    contact,
    prelude::{Contact, Session},
    session, user,
};
use feed::Notification;
use utility::UUID_NIL;

impl contact::Model {
    async fn from_user_and_ref_raw(
        user: Uuid,
        ref_user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Option<Self>, AppError> {
        Ok(Contact::find()
            .filter(
                Condition::all()
                    .add(contact::Column::User.eq(user))
                    .add(contact::Column::RefUser.eq(ref_user)),
            )
            .one(conn)
            .await?)
    }
    async fn from_user_and_ref(
        user: Uuid,
        ref_user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        Self::from_user_and_ref_raw(user, ref_user, conn)
            .await?
            .ok_or(AppError::NotFound(format!(
                "cannot find contact [{ref_user}] of [{user}]"
            )))
    }
    async fn is_user_and_ref_exist(
        user: Uuid,
        ref_user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<(), AppError> {
        if Self::from_user_and_ref_raw(user, ref_user, conn)
            .await?
            .is_some()
        {
            Err(AppError::Conflict(format!(
                "contact relation exist [{user}:{ref_user}]"
            )))
        } else {
            Ok(())
        }
    }
}

impl From<(Uuid, Uuid, Option<String>, Uuid)> for contact::ActiveModel {
    fn from((user, ref_user, alias, session): (Uuid, Uuid, Option<String>, Uuid)) -> Self {
        Self {
            id: ActiveValue::not_set(),
            user: ActiveValue::set(user),
            ref_user: ActiveValue::set(Some(ref_user)),
            alias: ActiveValue::set(alias),
            session: ActiveValue::set(session),
            created_at: ActiveValue::not_set(),
            category: ActiveValue::not_set(),
            pin: ActiveValue::not_set(),
            mute: ActiveValue::not_set(),
        }
    }
}

/// 发起添加好友
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/contact/new/{id}",
    params(
        ("id" = Uuid, Path, description = "要添加的用户主键")),
    responses(
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
    let user = payload.to_user(&state.conn).await?;
    let con = user::Model::from_uuid(contact, &state.conn).await?;
    if user.id == con.id {
        return Err(AppError::BadRequest("cannot add self".to_string()));
    }
    contact::Model::is_user_and_ref_exist(user.id, con.id, &state.conn).await?;
    contact::Model::is_user_and_ref_exist(con.id, user.id, &state.conn).await?;
    let s = session::ActiveModel::default();
    let s = Session::insert(s).exec(&state.conn).await?.last_insert_id;
    let c = contact::ActiveModel::from((user.id, con.id, con.alias.clone(), s));
    Contact::insert(c).exec(&state.conn).await?;
    event!(Level::DEBUG, "create new session [{}]", s);
    event!(Level::DEBUG, "user [{}] add [{}]", user.id, con.id);
    tokio::task::spawn(async move {
        if let Ok(data) = ContactList::query_new_contact(con, &state.conn).await {
            let data = Notification::ContactRequests { items: data };
            state
                .ws_pool
                .notify(
                    contact,
                    WebSocketMessage::Text(serde_json::to_string(&data).unwrap()),
                )
                .await;
        }
    });
    Ok(StatusCode::OK.into_response())
}

/// 拒绝好友申请
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/contact/reject/{id}",
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
    let c: contact::ActiveModel = contact::Model::from_user_and_ref(con, payload.id, &state.conn)
        .await?
        .into();
    contact::Model::is_user_and_ref_exist(payload.id, con, &state.conn).await?;
    Contact::delete(c).exec(&state.conn).await?;
    Ok(StatusCode::OK.into_response())
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct EditContactParams {
    /// 好友备注
    alias: Option<String>,
    /// 好友分组
    category: Option<String>,
    /// 置顶
    pin: Option<bool>,
    /// 静音
    mute: Option<bool>,
}

/// 修改好友
///
/// 备注, 分组, 置顶, 静音
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/contact/edit/{id}",
    params(
        ("id" = Uuid, Path, description = "要编辑的用户主键"),
        EditContactParams
    ),
    responses(
        (status = 200, description = "编辑成功")
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn edit_contact_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(con): Path<Uuid>,
    Query(params): Query<EditContactParams>,
) -> Result<impl IntoResponse, AppError> {
    let mut c: contact::ActiveModel =
        contact::Model::from_user_and_ref(payload.id, con, &state.conn)
            .await?
            .into();
    contact::Model::from_user_and_ref(con, payload.id, &state.conn).await?;
    c.alias = ActiveValue::set(params.alias);
    c.category = ActiveValue::set(params.category);
    c.pin = params
        .pin
        .map(|b| ActiveValue::set(b))
        .unwrap_or(ActiveValue::not_set());
    c.mute = params
        .mute
        .map(|b| ActiveValue::set(b))
        .unwrap_or(ActiveValue::not_set());
    Contact::update(c).exec(&state.conn).await?;
    Ok(StatusCode::OK.into_response())
}

/// 删除好友
#[cfg_attr(feature = "dev",
utoipa::path(
    delete,
    path = "/contact/delete/{id}",
    params(("id" = Uuid, Path, description = "要删除的用户主键")),
    responses(
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
    let mut c: contact::ActiveModel =
        contact::Model::from_user_and_ref(payload.id, con, &state.conn)
            .await?
            .into();
    let mut u: contact::ActiveModel =
        contact::Model::from_user_and_ref(con, payload.id, &state.conn)
            .await?
            .into();
    c.ref_user = ActiveValue::set(None);
    u.ref_user = ActiveValue::set(None);
    Contact::update(c).exec(&state.conn).await?;
    Contact::update(u).exec(&state.conn).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// 接受添加好友
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/contact/accept/{id}",
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
    let user = payload.to_user(&state.conn).await?;
    let con = user::Model::from_uuid(contact, &state.conn).await?;
    if user.id == con.id {
        return Err(AppError::BadRequest("cannot accept self".to_string()));
    }
    if contact::Model::from_user_and_ref_raw(user.id, con.id, &state.conn)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(format!(
            "contact already accepted [{}:{}]",
            user.id, con.id
        )));
    }
    let entry = contact::Model::from_user_and_ref(con.id, user.id, &state.conn).await?;
    let s = Session::find_by_id(entry.session)
        .one(&state.conn)
        .await?
        .ok_or(anyhow::anyhow!("session not found [{}]", entry.session))?;
    let c = contact::ActiveModel::from((user.id, con.id, con.alias, s.id));
    Contact::insert(c).exec(&state.conn).await?;
    tokio::task::spawn(async move {
        if let Ok(Some(c)) =
            contact::Model::from_user_and_ref_raw(con.id, user.id, &state.conn).await
        {
            let c: Chat = c.into();
            let data = Notification::ContactAccepts {
                items: ContactList {
                    num: 1,
                    items: vec![c],
                },
            };
            state
                .ws_pool
                .notify(
                    con.id,
                    WebSocketMessage::Text(serde_json::to_string(&data).unwrap()),
                )
                .await;
        }
    });
    Ok(StatusCode::OK.into_response())
}

/// 好友(申请)列表
///
/// 用于返回好友列表与返回好友申请
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(Deserialize))]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct ContactList {
    /// 好友(申请)数量
    #[cfg(test)]
    pub num: i32,
    #[cfg(not(test))]
    num: i32,
    /// 好友(申请者)的 UUID
    #[cfg(test)]
    pub items: Vec<Chat>,
    #[cfg(not(test))]
    items: Vec<Chat>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(Deserialize))]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct Chat {
    /// 好友(申请者)的 UUID
    ///
    /// 在通知中, 也可以表示群聊的 UUID
    #[cfg(test)]
    pub id: Uuid,
    #[cfg(not(test))]
    id: Uuid,
    /// 会话
    session: Uuid,
    /// 分类
    category: Option<String>,
    /// 备注
    alias: Option<String>,
    /// 是否置顶
    pin: bool,
    /// 是否静音
    mute: bool,
}

impl From<contact::Model> for Chat {
    fn from(c: contact::Model) -> Self {
        Self {
            id: c.ref_user.unwrap_or(*UUID_NIL),
            session: c.session,
            category: c.category,
            alias: c.alias,
            pin: c.pin,
            mute: c.mute,
        }
    }
}

#[derive(Debug, FromQueryResult)]
struct UserUuid {
    user: Uuid,
    session: Uuid,
    category: Option<String>,
    alias: Option<String>,
    pin: bool,
    mute: bool,
}

impl From<UserUuid> for Chat {
    fn from(u: UserUuid) -> Self {
        Self {
            id: u.user,
            session: u.session,
            category: u.category,
            alias: u.alias,
            pin: u.pin,
            mute: u.mute,
        }
    }
}

impl ContactList {
    async fn query_contact(user: user::Model, db: &DatabaseConnection) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
                    "SELECT a.ref_user AS user, a.session, a.category, a.alias, a.pin, a.mute FROM contact AS a INNER JOIN contact AS b ON a.user = b.ref_user AND a.ref_user = b.user WHERE a.user = $1",[user.id.into()])).all(db).await?;
        let items: Vec<Chat> = contacts.into_iter().map(UserUuid::into).collect();
        let num = items.len() as i32;
        Ok(Self { num, items })
    }

    async fn query_new_contact(
        user: user::Model,
        db: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
            "SELECT c.user, c.session, c.category, c.alias, c.pin, c.mute FROM contact AS c INNER JOIN (SELECT contact.user, contact.session FROM contact WHERE contact.ref_user = $1 EXCEPT SELECT contact.ref_user, contact.session FROM contact WHERE contact.user = $1) AS b ON c.user = b.user AND c.session = b.session",[user.id.into()])).all(db).await?;
        let items: Vec<Chat> = contacts
            .into_iter()
            .map(|c| {
                let mut c: Chat = c.into();
                c.category = None;
                c
            })
            .collect();
        let num = items.len() as i32;
        Ok(Self { num, items })
    }

    async fn query_pending_contact(
        user: user::Model,
        db: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let contacts:Vec<UserUuid> = UserUuid::find_by_statement(Statement::from_sql_and_values(Postgres,
                    "SELECT c.user, c.session, c.category, c.alias, c.pin, c.mute FROM contact AS c INNER JOIN (SELECT contact.ref_user AS user, contact.session FROM contact WHERE contact.user = $1 EXCEPT SELECT contact.user, contact.session FROM contact WHERE contact.ref_user = $1) AS b ON c.ref_user = b.user AND c.session = b.session;",[user.id.into()])).all(db).await?;
        let items: Vec<Chat> = contacts.into_iter().map(UserUuid::into).collect();
        let num = items.len() as i32;
        Ok(Self { num, items })
    }
}

/// 获取待通过好友列表
///
/// 即希望添加当前用户作为好友的用户列表
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/contact/new",
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
    let user = payload.to_user(&state.conn).await?;
    let data = ContactList::query_new_contact(user, &state.conn).await?;
    event!(
        Level::DEBUG,
        "get new contact list [{:?}] of user [{}]",
        data,
        payload.id
    );
    Ok(Json(data))
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct CategoryParams {
    /// 好友分组
    category: Option<String>,
}

/// 获取好友列表
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/contact/list",
    params(CategoryParams),
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
#[instrument(skip(state))]
pub async fn get_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Query(params): Query<CategoryParams>,
) -> Result<Json<ContactList>, AppError> {
    let user = payload.to_user(&state.conn).await?;
    let mut data = ContactList::query_contact(user, &state.conn).await?;
    if let Some(category) = params.category {
        let items = data
            .items
            .into_iter()
            .filter(|c| c.category.as_deref() == Some(category.as_str()))
            .collect();
        data.items = items;
        data.num = data.items.len() as i32;
    }
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
utoipa::path(
    get,
    path = "/contact/pending",
    responses(
        (status = 200, description = "获取成功", body = ContactList)
    ),
    tag = "contact"
))]
pub async fn get_pending_contacts_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<ContactList>, AppError> {
    let user = payload.to_user(&state.conn).await?;
    let data = ContactList::query_pending_contact(user, &state.conn).await?;
    event!(
        Level::DEBUG,
        "get pending contact list [{:?}] of user [{}]",
        data,
        payload.id
    );
    Ok(Json(data))
}

/// 获取好友分组列表
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/contact/category",
    responses(
        (status = 200, description = "获取成功", body = Vec<String>)
    ),
    tag = "contact"
))]
pub async fn get_categories_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<Vec<String>>, AppError> {
    let user = payload.to_user(&state.conn).await?;
    let categories = Contact::find()
        .filter(contact::Column::User.eq(user.id))
        .all(&state.conn)
        .await?;
    let mut categories = categories
        .into_iter()
        .filter(|c| c.category.is_some())
        .map(|c| c.category.unwrap())
        .collect::<Vec<String>>();
    categories.sort();
    categories.dedup();
    Ok(Json(categories))
}
