# MindCache

**MindCache** is a high-performance memory storage and retrieval system built for AI applications, chatbots, and intelligent agents. It enables **context persistence**, **intelligent memory decay**, **fast recall**, and **multi-session memory intelligence**, making it the ideal memory backend for AI workflows.


[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)  
[ View Roadmap](#️-roadmap) | [ API Docs](#-api-reference) | [ Get Started](#-quick-start) | [🔧 Deployment](#️-deployment)

---

## Why MindCache?

Modern AI agents struggle to maintain memory across sessions and scale with context. MindCache solves this with:

**Persistent Memory**: AI agents can remember across interactions  
**Fast Recall**: Built in Rust for low-latency lookups  
**Memory Decay**: Old or low-importance memories fade away  
**Human-Like Context Handling**: Organize, filter, and search by topic, session, or importance  
**Plug-and-Play**: Integrate via REST API, JavaScript SDK, or CLI  
**Enterprise-Ready**: API key auth, rate limiting, backups, logging, and Prometheus metrics  
**No Vendor Lock-in**: Fully local, Docker-ready, FFI-free setup

> **Use Cases**: LLM memory backends · Chatbots · Journaling agents · Learning assistants · Meeting memory · Personal knowledge bases

---

## Features

### Core Memory Engine
- Persistent storage with TTL and importance scoring
- Natural language recall with filters and search
- Session-aware memory management
- Bulk operations and summaries

### Performance
- Rust-powered core for ultra-fast read/write
- Efficient local-first storage with compression
- Thread-safe and concurrent memory access

### Developer Experience
- Clean REST API
- CLI for scripting and terminal use
- JavaScript SDK (Node.js + browser)
- Fully typed (TypeScript)
- Easy Docker + PM2 deployment

### Enterprise-Ready
- API key-based authentication
- Rate limiting and logging
- Health metrics and Prometheus integration
- Backup and export tools

---

## Architecture

           ┌────────────────────┐
           │   CLI / SDK / App  │
           └────────┬───────────┘
                    │
                    ▼
           ┌────────────────────┐
           │   Node.js API      │   ← RESTful endpoints (Express)
           └────────┬───────────┘
                    │  FFI Bridge / IPC
                    ▼
           ┌────────────────────┐
           │     Rust Core      │   ← Memory engine: store, decay, search
           └────────┬───────────┘
                    │
                    ▼
           ┌────────────────────┐
           │   Local Storage    │   ← Indexed and compressed
           └────────────────────┘



---

## Quick Start
 
# 1. Clone the repo
git clone https://github.com/hamzzaaamalik/mindcache.git
cd mindcache

# 2. Start the API
cd node-api
npm install
npm start

# 3. Test it
curl http://localhost:3000/health

# 4. Use the CLI
cd ../cli
npm install
node mindcache.js ping


### API Reference
Base URL: http://localhost:3000

## Health Check


           GET /health

## Save Memory

           POST /api/memory/save
           Content-Type: application/json
           
           {
             "userId": "user_123",
             "sessionId": "session_456",
             "content": "I learned about memory decay",
             "importance": 0.8,
             "ttlHours": 720
           }

## Recall Memories

           POST /api/memory/recall
           Content-Type: application/json
           
           {
             "userId": "user_123",
             "query": "memory",
             "limit": 10
           }
## Summarize Session

           POST /api/memory/summarize
           Content-Type: application/json
           
           {
             "sessionId": "session_456"
           }



### CLI Usage


# Ping
mindcache ping

# Save memory
mindcache save --user alice --session session1 --content "AI notes"

# Recall memory
mindcache recall --user alice --query "AI"

# Session management
mindcache sessions --user alice --create --name "Meeting Notes"

# Summarize
mindcache summarize --session session1

### SDK Usage (JavaScript)

const { MindCacheSDK } = require('mindcache-sdk');

const mindcache = new MindCacheSDK();

await mindcache.saveMemory({
  userId: 'u1',
  sessionId: 's1',
  content: 'Studied reinforcement learning',
  importance: 0.9
});

const context = await mindcache.recallMemories({
  userId: 'u1',
  query: 'reinforcement'
});

Examples
AI Chatbot Memory
Learning Tracker
Meeting Notes Agent

All available in the /examples folder.

Deployment
Option 1: PM2 (Recommended)
bash
Copy
Edit
npm install -g pm2

pm2 start ecosystem.config.js --env production
pm2 save
pm2 startup
Option 2: Docker

docker-compose up -d
Option 3: Kubernetes

# See k8s-deployment.yaml in repo


### Monitoring & Observability
Health Check: GET /health

Metrics: Prometheus-compatible export

Logs: Winston (file + console)

Alerts: Custom thresholds configurable

🛠️ Troubleshooting
Issue	Fix
ffi-napi errors	Use Docker or install Rust manually
No data recalled	Check TTL, session ID, importance, decay settings
CLI not working	Use npm link or node mindcache.js
API down	Ensure npm start and port 3000 is active

🤝 Contributing
We welcome contributions!


git clone https://github.com/hamzzaaamalik/mindcache.git
git checkout -b feature/your-feature
npm run dev
npm test
Follow conventional commits
Add tests and update docs before PR

📄 License
MIT — see LICENSE

    Roadmap
    v1.1.0
Memory TTL and Decay

Importance scoring

Bulk save + Summarize sessions

    v1.2.0
Vector embedding support

GraphQL API

Multi-language SDKs

Real-time sync across nodes

    v2.0.0
Distributed cluster architecture

Advanced security and ACLs

Enterprise monitoring dashboard

    Built By
Malik Ameer Hamza Khan
Empowering AI agents with intelligent memory.

    Support
GitHub Issues

Docs (coming soon)

Email: support@mindcache.dev
