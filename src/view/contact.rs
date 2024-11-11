use super::*;
use entity::{
    contact,
    prelude::{Contact, Session, User},
    session, user,
};

/// 发起添加好友
#[cfg_attr(feature = "dev",
utoipa::path(post, path = "/contact",
    params(
        ("id" = Uuid, Path, description = "要添加的用户主键")
    ),
    responses(
        (status = 200, description = "发起成功")
    ),
    tag = "user"
))]
#[instrument(skip(state))]
pub async fn add_contact_handler(
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
