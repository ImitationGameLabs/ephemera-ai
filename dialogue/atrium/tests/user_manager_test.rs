mod fixtures;

use fixtures::{create_user_manager, setup_test_db};
use atrium::db::{CreateUserDto, UpdateUserDto, UserError};

/// 测试用户创建和认证场景
#[tokio::test]
async fn test_user_creation_and_auth() {
    let (_container, db) = setup_test_db().await;
    let manager = create_user_manager(&db);

    // 创建用户成功
    let dto = CreateUserDto {
        name: "alice".to_string(),
        bio: "Alice's bio".to_string(),
        password: "secret123".to_string(),
    };
    let user = manager.create_user(&dto).await.unwrap();
    assert_eq!(user.name, "alice");
    assert_eq!(user.bio, "Alice's bio");
    assert_eq!(user.message_height, 0);
    assert!(!user.status.online);

    // 重复创建同名用户失败
    let result = manager.create_user(&dto).await;
    assert!(matches!(result, Err(UserError::UserAlreadyExists(_))));

    // 认证 - 正确密码
    let auth_user = manager.authenticate_user("alice", "secret123").await.unwrap();
    assert_eq!(auth_user.name, "alice");

    // 认证 - 错误密码
    let result = manager.authenticate_user("alice", "wrong").await;
    assert!(matches!(result, Err(UserError::InvalidPassword(_))));

    // 认证 - 不存在的用户（返回 InvalidPassword 防止用户枚举）
    let result = manager.authenticate_user("nonexistent", "any").await;
    assert!(matches!(result, Err(UserError::InvalidPassword(_))));
}

/// 测试用户更新场景
#[tokio::test]
async fn test_user_update() {
    let (_container, db) = setup_test_db().await;
    let manager = create_user_manager(&db);

    // Setup: 创建用户
    let dto = CreateUserDto {
        name: "bob".to_string(),
        bio: "Old bio".to_string(),
        password: "old_password".to_string(),
    };
    manager.create_user(&dto).await.unwrap();

    // 更新 bio
    let updated = manager.update_user("bob", &UpdateUserDto {
        bio: Some("New bio".to_string()),
        new_password: None,
    }).await.unwrap();
    assert_eq!(updated.bio, "New bio");

    // 更新密码
    manager.update_user("bob", &UpdateUserDto {
        bio: None,
        new_password: Some("new_password".to_string()),
    }).await.unwrap();

    // 旧密码失效，新密码有效
    assert!(manager.authenticate_user("bob", "old_password").await.is_err());
    assert!(manager.authenticate_user("bob", "new_password").await.is_ok());

    // 更新不存在的用户失败
    let result = manager.update_user("nonexistent", &UpdateUserDto {
        bio: Some("bio".to_string()),
        new_password: None,
    }).await;
    assert!(matches!(result, Err(UserError::UserNotFound(_))));
}

/// 测试用户列表和状态功能
#[tokio::test]
async fn test_user_list_and_heartbeat() {
    let (_container, db) = setup_test_db().await;
    let manager = create_user_manager(&db);

    // 空列表
    let users = manager.get_all_users().await.unwrap();
    assert!(users.is_empty());

    // 创建多个用户（非字母序）
    for name in ["charlie", "alice", "bob"] {
        manager.create_user(&CreateUserDto {
            name: name.to_string(),
            bio: format!("{}'s bio", name),
            password: "pass".to_string(),
        }).await.unwrap();
    }

    // 列表按名字排序
    let users = manager.get_all_users().await.unwrap();
    assert_eq!(users.len(), 3);
    assert_eq!(users[0].name, "alice");
    assert_eq!(users[1].name, "bob");
    assert_eq!(users[2].name, "charlie");

    // heartbeat 更新在线状态
    let before = manager.get_user_by_name("alice").await.unwrap();
    assert!(!before.status.online);

    let after = manager.update_heartbeat("alice").await.unwrap();
    assert!(after.status.online);
    assert!(after.status.last_seen.is_some());

    // message_height 更新
    let updated = manager.update_message_height("alice", 42).await.unwrap();
    assert_eq!(updated.message_height, 42);

    // 获取用户验证持久化
    let user = manager.get_user_by_name("alice").await.unwrap();
    assert_eq!(user.message_height, 42);

    // 获取不存在的用户失败
    let result = manager.get_user_by_name("nonexistent").await;
    assert!(matches!(result, Err(UserError::UserNotFound(_))));

    // heartbeat 不存在的用户失败
    let result = manager.update_heartbeat("nonexistent").await;
    assert!(matches!(result, Err(UserError::UserNotFound(_))));
}
