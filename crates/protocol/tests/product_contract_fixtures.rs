use protocol::{
    CommandResult, Envelope, ProductClientMessage, ProductCommand, ProductCommandResponse,
    ProductEvent, ProductEventPage, ProductProject, ProductServerMessage, ProductTicket,
    ProductTicketPriority, ProductUser, ProductWorkspace, ProductWorkspaceRole, ResponseEnvelope,
};
use uuid::Uuid;

#[test]
fn all_product_transport_shapes_round_trip() {
    let user_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();
    let ticket_id = Uuid::new_v4();
    let user = ProductUser {
        id: user_id,
        email: "dev@jet-black.local".to_owned(),
        display_name: "Developer".to_owned(),
        version: 1,
    };
    let workspace = ProductWorkspace {
        id: workspace_id,
        slug: "jet-black".to_owned(),
        name: "Jet Black".to_owned(),
        role: ProductWorkspaceRole::Owner,
        version: 1,
    };
    let project = ProductProject {
        id: project_id,
        workspace_id,
        identifier: "JB".to_owned(),
        name: "Jet Black".to_owned(),
        description: "Agentic delivery".to_owned(),
        repository_identity: Some("github:example/jet-black".to_owned()),
        version: 1,
    };
    let ticket = ProductTicket {
        id: ticket_id,
        project_id,
        sequence_number: 1,
        title: "Build the transport".to_owned(),
        description: String::new(),
        state_id: None,
        priority: ProductTicketPriority::High,
        created_by_id: user_id,
        version: 1,
    };
    let event = ProductEvent {
        cursor: 1,
        workspace_id,
        aggregate_kind: "ticket".to_owned(),
        aggregate_id: ticket_id,
        aggregate_version: 1,
        event_kind: "ticket.created".to_owned(),
        actor_id: Some(user_id),
        body: format!(r#"{{"ticket_id":"{ticket_id}"}}"#),
        created_at_ms: 100,
    };
    let client_messages = [
        ProductClientMessage::Subscribe {
            workspace_id,
            after_cursor: 0,
        },
        ProductClientMessage::Command(Envelope::new(ProductCommand::CreateWorkspace {
            slug: workspace.slug.clone(),
            name: workspace.name.clone(),
        })),
        ProductClientMessage::Command(Envelope::new(ProductCommand::CreateProject {
            workspace_id,
            identifier: project.identifier.clone(),
            name: project.name.clone(),
            description: project.description.clone(),
            repository_identity: project.repository_identity.clone(),
        })),
        ProductClientMessage::Command(Envelope::new(ProductCommand::CreateTicket {
            project_id,
            title: ticket.title.clone(),
            description: ticket.description.clone(),
            priority: ticket.priority,
            idempotency_key: "fixture-ticket".to_owned(),
        })),
        ProductClientMessage::Acknowledge { cursor: 1 },
        ProductClientMessage::Ping,
    ];
    let server_messages = [
        ProductServerMessage::Ready {
            session_id: Uuid::new_v4(),
        },
        ProductServerMessage::CommandResult(ResponseEnvelope::success(
            Uuid::new_v4(),
            ProductCommandResponse::WorkspaceCreated(workspace),
        )),
        ProductServerMessage::CommandResult(ResponseEnvelope::success(
            Uuid::new_v4(),
            ProductCommandResponse::ProjectCreated(project),
        )),
        ProductServerMessage::CommandResult(ResponseEnvelope::success(
            Uuid::new_v4(),
            ProductCommandResponse::TicketCreated(ticket),
        )),
        ProductServerMessage::Events(ProductEventPage {
            events: vec![event],
            next_cursor: 1,
            has_more: false,
        }),
        ProductServerMessage::Subscribed {
            workspace_id,
            cursor: 1,
        },
        ProductServerMessage::Pong,
    ];

    for message in client_messages {
        let json = serde_json::to_string(&message).expect("serialize client message");
        assert_eq!(
            serde_json::from_str::<ProductClientMessage>(&json).expect("client message"),
            message
        );
    }
    for message in server_messages {
        let json = serde_json::to_string(&message).expect("serialize server message");
        assert_eq!(
            serde_json::from_str::<ProductServerMessage>(&json).expect("server message"),
            message
        );
    }

    let response = ResponseEnvelope::success(
        Uuid::new_v4(),
        ProductCommandResponse::TicketCreated(ProductTicket {
            id: ticket_id,
            project_id,
            sequence_number: 1,
            title: "Build the transport".to_owned(),
            description: String::new(),
            state_id: None,
            priority: ProductTicketPriority::High,
            created_by_id: user.id,
            version: 1,
        }),
    );
    assert!(matches!(response.result, CommandResult::Ok(_)));
}
