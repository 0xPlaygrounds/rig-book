use rig::{
    client::{CompletionClient, ProviderClient},
    completion::{Prompt, PromptError},
};
use tokio::time::{Duration, interval};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Manager-worker pattern example\n---");
    manager_worker_agent().await?;

    println!("Swarm agent example\n---");
    swarm_agent_example().await?;

    Ok(())
}

/// An example to showcase the manager-worker agent pattern using two agents named Bob and Alice.
/// Note that this may take a while to execute due to awaiting multiple LLM responses sequentially (can't stream a response directly into another input).
async fn manager_worker_agent() -> Result<(), Box<dyn std::error::Error>> {
    let openai_client = rig::providers::openai::Client::from_env();

    let prompt = "Ask Bob to write an email for you and let me know what he has written.";
    println!("Prompt: {prompt}");

    let bob = openai_client.agent("gpt-5")
        .name("Bob")
        .description("An employee who works in admin at FooBar Inc.")
        .preamble("You are Bob, an employee working in admin at FooBar Inc. Alice, your manager, may ask you to do things. You need to do them.")
        .build();

    let alice = openai_client
        .agent("gpt-5")
        .name("Alice")
        .description("A manager at FooBar Inc.")
        .preamble("You are a manager in the admin department at FooBar Inc. You manage Bob.")
        .tool(bob)
        .build();

    let res = alice
        .prompt("Ask Bob to write an email for you and let me know what he has written.")
        .await?;

    println!("Response: {res}");

    Ok(())
}

use rig::providers::openai;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;

/// Message types for inter-agent communication
#[derive(Debug, Clone)]
enum AgentMessage {
    Task(String),
    Response(String, String), // (from_agent_id, content)
    Trigger(String),
    Shutdown,
}

/// Agent state
struct AgentState {
    task_queue: Vec<String>,
    conversation_history: Vec<String>,
}

/// Actor-based autonomous agent
struct AutonomousAgent {
    id: String,
    client: openai::Client,
    state: Arc<RwLock<AgentState>>,
    inbox: mpsc::Receiver<AgentMessage>,
    peer_channels: Arc<RwLock<Vec<mpsc::Sender<AgentMessage>>>>,
}

impl AutonomousAgent {
    fn new(id: String, api_key: String, inbox: mpsc::Receiver<AgentMessage>) -> Self {
        let client = openai::Client::new(&api_key).unwrap();
        let state = Arc::new(RwLock::new(AgentState {
            task_queue: Vec::new(),
            conversation_history: Vec::new(),
        }));

        Self {
            id,
            client,
            state,
            inbox,
            peer_channels: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register peer agents for communication
    async fn register_peer(&self, peer_channel: mpsc::Sender<AgentMessage>) {
        let mut peers = self.peer_channels.write().await;
        peers.push(peer_channel);
    }

    /// Send message to all peer agents
    async fn broadcast_to_peers(&self, message: AgentMessage) {
        let peers = self.peer_channels.read().await;
        for peer in peers.iter() {
            let _ = peer.send(message.clone()).await;
        }
    }

    /// Process autonomous task using LLM
    /// This currently shows a simple LLM prompt, but if you wanted you could give your agent some tools!
    async fn process_autonomous_task(&self, task: &str) -> Result<String, PromptError> {
        let agent = self
            .client
            .agent("gpt-5")
            .preamble(&format!(
                "Your name is {}. Process tasks autonomously and coordinate with other agents.",
                self.id
            ))
            .build();

        let response = agent.prompt(task).await?;
        Ok(response)
    }

    async fn handle_message(&self, task: AgentMessage) {
        match task {
            AgentMessage::Task(task) => {
                println!("[{}] Received task: {}", self.id, task);

                match self.process_autonomous_task(&task).await {
                    Ok(result) => {
                        println!("[{}] Completed task: {}", self.id, result);

                        // Store in history
                        let mut state = self.state.write().await;
                        state
                            .conversation_history
                            .push(format!("Task: {} | Result: {}", task, result));

                        // Broadcast result to peers
                        self.broadcast_to_peers(AgentMessage::Response(self.id.clone(), result))
                            .await;
                    }
                    Err(e) => eprintln!("[{}] Error processing task: {}", self.id, e),
                }
            }
            AgentMessage::Response(from_id, content) => {
                println!(
                    "[{}] Received response from {}: {}",
                    self.id, from_id, content
                );
                let mut state = self.state.write().await;
                state
                    .conversation_history
                    .push(format!("From {}: {}", from_id, content));
            }
            AgentMessage::Trigger(trigger_msg) => {
                println!("[{}] External trigger: {}", self.id, trigger_msg);
                // Process trigger autonomously
                let _ = self.process_autonomous_task(&trigger_msg).await;
            }
            message => {
                println!("Unsupported message variant received: {message:?}");
                // this could theoretically return an error or panic
                // this should never return the shutdown enum variant because enums are eagerly evaluated
            }
        }
    }

    // Main actor loop
    async fn run(mut self) {
        println!("Agent '{}' started and running autonomously", self.id);

        // External trigger: periodic self-check (runs every 10 seconds)
        let mut tick_interval = interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                // Handle incoming messages from other agents
                Some(msg) = self.inbox.recv() => {
                    match msg {
                        AgentMessage::Shutdown => {
                            println!("Shutting down...");
                            break
                        }
                        _ => {
                            self.handle_message(msg).await;
                        }
                    }
                }
                // Autonomous periodic task (external trigger)
                _ = tick_interval.tick() => {
                    println!("[{}] Autonomous tick - checking for self-initiated tasks", self.id);

                    // Check if agent should create its own task
                    // Use scoped brackets here to avoid needing to manually drop lock
                    let needs_to_create_own_task =  {
                        let state_rlock = self.state.read().await;
                        state_rlock.task_queue.is_empty() && !state_rlock.conversation_history.is_empty()
                    };

                    // Check if agent should create its own task
                    if needs_to_create_own_task {
                        let summary_task = "Summarize what you've accomplished so far in one sentence.";
                        match self.process_autonomous_task(summary_task).await {
                            Ok(summary) => {
                                println!("[{}] Self-initiated summary: {}", self.id, summary);
                            }
                            Err(e) => eprintln!("[{}] Error in autonomous task: {}", self.id, e),
                        }
                    }
                }
            }
        }
    }
}

async fn swarm_agent_example() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    // Create channels for agent communication
    let (tx1, rx1) = mpsc::channel(100);
    let (tx2, rx2) = mpsc::channel(100);
    let (tx3, rx3) = mpsc::channel(100);

    // Create agents
    let agent1 = AutonomousAgent::new("Tom".to_string(), api_key.clone(), rx1);
    let agent2 = AutonomousAgent::new("Richard".to_string(), api_key.clone(), rx2);
    let agent3 = AutonomousAgent::new("Harry".to_string(), api_key, rx3);

    // Register peers (each agent knows about the others)
    agent1.register_peer(tx2.clone()).await;
    agent1.register_peer(tx3.clone()).await;
    agent2.register_peer(tx1.clone()).await;
    agent2.register_peer(tx3.clone()).await;
    agent3.register_peer(tx1.clone()).await;
    agent3.register_peer(tx2.clone()).await;

    // Spawn agent actors
    let handle1 = tokio::spawn(agent1.run());
    let handle2 = tokio::spawn(agent2.run());
    let handle3 = tokio::spawn(agent3.run());

    // Send initial task to Agent-Alpha
    tx1.send(AgentMessage::Task(
        "Analyze the benefits of autonomous agent systems".to_string(),
    ))
    .await?;

    // External trigger example
    tokio::time::sleep(Duration::from_secs(5)).await;
    tx2.send(AgentMessage::Trigger(
        "Check system status and report findings".to_string(),
    ))
    .await?;

    // Let agents run for demonstration
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Shutdown
    tx1.send(AgentMessage::Shutdown).await?;
    tx2.send(AgentMessage::Shutdown).await?;
    tx3.send(AgentMessage::Shutdown).await?;

    handle1.await?;
    handle2.await?;
    handle3.await?;

    Ok(())
}
