mod fixtures;

use atrium::db::message_manager::{CreateMessageDto, MessageError};
use atrium::db::user_manager::CreateUserDto;
use fixtures::{create_message_manager, create_user_manager, setup_test_db};

/// 测试消息 CRUD 场景
#[tokio::test]
async fn test_message_crud() {
    let (_container, db) = setup_test_db().await;
    let user_manager = create_user_manager(&db);
    let message_manager = create_message_manager(&db);

    // Setup: 创建用户
    let user_dto = CreateUserDto {
        name: "alice".to_string(),
        bio: "".to_string(),
        password: "pass".to_string(),
    };
    user_manager.create_user(&user_dto).await.unwrap();

    // 创建消息
    let dto =
        CreateMessageDto { content: "Hello, world!".to_string(), sender: "alice".to_string() };
    let message = message_manager.create_message(&dto).await.unwrap();
    assert!(message.id > 0);
    assert_eq!(message.content, "Hello, world!");
    assert_eq!(message.sender, "alice");

    // 获取消息
    let retrieved = message_manager.get_message(message.id).await.unwrap();
    assert_eq!(retrieved.id, message.id);
    assert_eq!(retrieved.content, "Hello, world!");

    // 获取不存在的消息失败
    let result = message_manager.get_message(999999).await;
    assert!(matches!(result, Err(MessageError::MessageNotFound(id)) if id == 999999));

    // 删除消息成功
    message_manager.delete_message(message.id).await.unwrap();
    let result = message_manager.get_message(message.id).await;
    assert!(matches!(result, Err(MessageError::MessageNotFound(_))));

    // 删除不存在的消息失败
    let result = message_manager.delete_message(999999).await;
    assert!(matches!(result, Err(MessageError::MessageNotFound(_))));
}

/// 测试消息查询场景
#[tokio::test]
async fn test_message_query() {
    let (_container, db) = setup_test_db().await;
    let user_manager = create_user_manager(&db);
    let message_manager = create_message_manager(&db);

    // Setup: 创建用户
    for name in ["alice", "bob"] {
        user_manager
            .create_user(&CreateUserDto {
                name: name.to_string(),
                bio: "".to_string(),
                password: "pass".to_string(),
            })
            .await
            .unwrap();
    }

    // 空列表
    let messages = message_manager
        .get_messages(None, None, None)
        .await
        .unwrap();
    assert!(messages.is_empty());

    // 创建多条消息（alice 3条，bob 2条）
    for i in 0..3 {
        message_manager
            .create_message(&CreateMessageDto {
                content: format!("Alice message {}", i),
                sender: "alice".to_string(),
            })
            .await
            .unwrap();
    }
    for i in 0..2 {
        message_manager
            .create_message(&CreateMessageDto {
                content: format!("Bob message {}", i),
                sender: "bob".to_string(),
            })
            .await
            .unwrap();
    }

    // sender 过滤
    let alice_messages = message_manager
        .get_messages(Some("alice"), None, None)
        .await
        .unwrap();
    assert_eq!(alice_messages.len(), 3);
    assert!(alice_messages.iter().all(|m| m.sender == "alice"));

    // 分页
    let first_page = message_manager
        .get_messages(None, Some(3), Some(0))
        .await
        .unwrap();
    assert_eq!(first_page.len(), 3);

    let second_page = message_manager
        .get_messages(None, Some(3), Some(3))
        .await
        .unwrap();
    assert_eq!(second_page.len(), 2);

    // 按时间倒序（最新的在前）
    let messages = message_manager
        .get_messages(None, None, None)
        .await
        .unwrap();
    assert!(messages[0].created_at >= messages[1].created_at);
}

/// 测试消息增量同步场景
#[tokio::test]
async fn test_message_incremental_sync() {
    let (_container, db) = setup_test_db().await;
    let user_manager = create_user_manager(&db);
    let message_manager = create_message_manager(&db);

    // Setup
    user_manager
        .create_user(&CreateUserDto {
            name: "alice".to_string(),
            bio: "".to_string(),
            password: "pass".to_string(),
        })
        .await
        .unwrap();

    // 空表时 latest_id 为 None
    let latest = message_manager.get_latest_message_id().await.unwrap();
    assert!(latest.is_none());

    // 创建 5 条消息
    let mut ids = Vec::new();
    for i in 0..5 {
        let msg = message_manager
            .create_message(&CreateMessageDto {
                content: format!("Message {}", i),
                sender: "alice".to_string(),
            })
            .await
            .unwrap();
        ids.push(msg.id);
    }

    // 获取 latest_id
    let latest = message_manager.get_latest_message_id().await.unwrap();
    assert_eq!(latest, Some(ids[4]));

    // since_id: 获取第二条之后的消息
    let new_messages = message_manager
        .get_messages_since_id(ids[1], None)
        .await
        .unwrap();
    assert_eq!(new_messages.len(), 3);
    assert!(new_messages.iter().all(|m| m.id > ids[1]));
    // 按ID升序
    assert_eq!(new_messages[0].id, ids[2]);
    assert_eq!(new_messages[1].id, ids[3]);
    assert_eq!(new_messages[2].id, ids[4]);

    // since_id 带 limit
    let limited = message_manager
        .get_messages_since_id(ids[1], Some(2))
        .await
        .unwrap();
    assert_eq!(limited.len(), 2);

    // since_id 为最新 id 时返回空
    let empty = message_manager
        .get_messages_since_id(ids[4], None)
        .await
        .unwrap();
    assert!(empty.is_empty());
}
