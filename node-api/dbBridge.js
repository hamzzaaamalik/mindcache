const ffi = require('ffi-napi')
const ref = require('ref-napi')
const path = require('path')
const fs = require('fs')

/**
 * Rust Bridge - FFI interface to MindCache Rust core
 *
 * This module provides a JavaScript interface to the Rust storage engine
 * using Node.js FFI (Foreign Function Interface)
 */

class RustBridge {
  constructor (config = {}) {
    this.config = {
      storage_path: config.storage_path || './mindcache_data',
      auto_decay_enabled: config.auto_decay_enabled !== false,
      decay_interval_hours: config.decay_interval_hours || 24,
      default_memory_ttl_hours: config.default_memory_ttl_hours || 720,
      enable_compression: config.enable_compression !== false,
      max_memories_per_user: config.max_memories_per_user || 10000,
      importance_threshold: config.importance_threshold || 0.3
    }

    this.rustLib = null
    this.cachePtr = null
    this.isInitialized = false
  }

  /**
     * Initialize the Rust bridge and load the native library
     */
  async initialize () {
    try {
      console.log('🔗 Initializing Rust bridge...')

      // Load the Rust library
      await this.loadRustLibrary()

      // Initialize MindCache with configuration
      await this.initializeMindCache()

      this.isInitialized = true
      console.log('✅ Rust bridge initialized successfully')
    } catch (error) {
      console.error('❌ Failed to initialize Rust bridge:', error)
      throw new Error(`Rust bridge initialization failed: ${error.message}`)
    }
  }

  /**
     * Load the Rust native library using FFI
     */
  async loadRustLibrary () {
    // Determine library path based on platform
    const libPath = this.getLibraryPath()

    console.log(`📚 Loading Rust library from: ${libPath}`)

    // Verify library exists
    if (!fs.existsSync(libPath)) {
      throw new Error(`Rust library not found at: ${libPath}. Please build the Rust core first.`)
    }

    // Define FFI interface
    this.rustLib = ffi.Library(libPath, {
      // Core functions
      mindcache_init: ['pointer', []],
      mindcache_init_with_config: ['pointer', ['string']],
      mindcache_destroy: ['void', ['pointer']],

      // Memory operations
      mindcache_save: ['string', ['pointer', 'string', 'string', 'string', 'string']],
      mindcache_recall: ['string', ['pointer', 'string', 'string', 'string', 'int']],
      mindcache_summarize: ['string', ['pointer', 'string']],
      mindcache_decay: ['string', ['pointer']],
      mindcache_get_stats: ['string', ['pointer']],

      // Utility functions
      mindcache_free_string: ['void', ['string']]
    })

    console.log('✅ Rust library loaded successfully')
  }

  /**
     * Get the correct library path for the current platform
     */
  getLibraryPath () {
    const platform = process.platform

    // Fix the base directory path - it should be relative to node-api folder
    const baseDir = path.join(__dirname, '../rust-core/target')

    let libName

    switch (platform) {
      case 'win32':
        libName = 'mindcache_core.dll'
        break
      case 'darwin':
        libName = 'libmindcache_core.dylib'
        break
      case 'linux':
        libName = 'libmindcache_core.so'
        break
      default:
        throw new Error(`Unsupported platform: ${platform}`)
    }

    // Try different build configurations
    const possiblePaths = [
      path.join(baseDir, 'release', libName),
      path.join(baseDir, 'debug', libName),
      path.join(__dirname, libName), // Current directory fallback
      path.join(__dirname, '..', 'rust-core', 'target', 'release', libName),
      path.join(__dirname, '..', 'rust-core', 'target', 'debug', libName)
    ]

    console.log('🔍 Searching for Rust library...')
    for (const libPath of possiblePaths) {
      console.log(`   Checking: ${libPath}`)
      if (fs.existsSync(libPath)) {
        console.log(`✅ Found Rust library: ${libPath}`)
        return libPath
      }
    }

    // List what files actually exist in target directories
    const releaseDir = path.join(baseDir, 'release')
    const debugDir = path.join(baseDir, 'debug')

    console.log('\n📁 Available files:')
    if (fs.existsSync(releaseDir)) {
      const releaseFiles = fs.readdirSync(releaseDir)
      console.log(`   Release dir: ${releaseFiles.join(', ')}`)
    } else {
      console.log(`   Release dir does not exist: ${releaseDir}`)
    }

    if (fs.existsSync(debugDir)) {
      const debugFiles = fs.readdirSync(debugDir)
      console.log(`   Debug dir: ${debugFiles.join(', ')}`)
    } else {
      console.log(`   Debug dir does not exist: ${debugDir}`)
    }

    throw new Error(`Rust library not found. Tried: ${possiblePaths.join(', ')}\n\nPlease build the Rust core first:\n  cd rust-core\n  cargo build --release`)
  }

  /**
     * Initialize MindCache with configuration
     */
  async initializeMindCache () {
    const configJson = JSON.stringify(this.config)
    console.log('⚙️ Initializing MindCache with config:', configJson)

    // Initialize with configuration
    this.cachePtr = this.rustLib.mindcache_init_with_config(configJson)

    if (this.cachePtr.isNull()) {
      // Fallback to default initialization
      console.warn('⚠️ Config initialization failed, trying default...')
      this.cachePtr = this.rustLib.mindcache_init()

      if (this.cachePtr.isNull()) {
        throw new Error('Failed to initialize MindCache')
      }
    }

    console.log('✅ MindCache core initialized')
  }

  /**
     * Save a memory item
     */
  async saveMemory ({ userId, sessionId, content, metadata = {}, importance = 0.5, ttlHours = null }) {
    this.ensureInitialized()

    try {
      const metadataJson = JSON.stringify(metadata)

      console.log(`💾 Saving memory for user ${userId}, session ${sessionId}`)

      const result = this.rustLib.mindcache_save(
        this.cachePtr,
        userId,
        sessionId,
        content,
        metadataJson
      )

      if (!result) {
        throw new Error('Failed to save memory - no result returned')
      }

      // The result is the memory ID
      console.log(`✅ Memory saved with ID: ${result}`)
      return result
    } catch (error) {
      console.error('❌ Error saving memory:', error)
      throw new Error(`Failed to save memory: ${error.message}`)
    }
  }

  /**
     * Recall memories with filters
     */
  async recallMemories (filter) {
    this.ensureInitialized()

    try {
      const {
        userId,
        query = null,
        sessionId = null,
        limit = 50
      } = filter

      console.log(`🔍 Recalling memories for user ${userId}${query ? ` with query "${query}"` : ''}`)

      const result = this.rustLib.mindcache_recall(
        this.cachePtr,
        userId,
        query,
        sessionId,
        limit
      )

      if (!result) {
        return []
      }

      // Parse the JSON result
      const memories = JSON.parse(result)
      console.log(`✅ Recalled ${memories.length} memories`)

      return memories
    } catch (error) {
      console.error('❌ Error recalling memories:', error)
      throw new Error(`Failed to recall memories: ${error.message}`)
    }
  }

  /**
     * Generate session summary
     */
  async summarizeSession (sessionId) {
    this.ensureInitialized()

    try {
      console.log(`📋 Generating summary for session ${sessionId}`)

      const result = this.rustLib.mindcache_summarize(this.cachePtr, sessionId)

      if (!result) {
        throw new Error('No summary generated')
      }

      // Parse the JSON result
      const summary = JSON.parse(result)
      console.log(`✅ Summary generated for session ${sessionId}`)

      return summary
    } catch (error) {
      console.error('❌ Error generating summary:', error)
      throw new Error(`Failed to generate summary: ${error.message}`)
    }
  }

  /**
     * Get system statistics
     */
  async getStats () {
    this.ensureInitialized()

    try {
      console.log('📊 Getting system statistics')

      const result = this.rustLib.mindcache_get_stats(this.cachePtr)

      if (!result) {
        return {}
      }

      // Parse the JSON result
      const stats = JSON.parse(result)
      console.log('✅ Statistics retrieved')

      return stats
    } catch (error) {
      console.error('❌ Error getting statistics:', error)
      throw new Error(`Failed to get statistics: ${error.message}`)
    }
  }

  /**
     * Run memory decay process
     */
  async runDecay (force = false) {
    this.ensureInitialized()

    try {
      console.log(`🧹 Running memory decay process${force ? ' (forced)' : ''}`)

      const result = this.rustLib.mindcache_decay(this.cachePtr)

      if (!result) {
        throw new Error('No decay stats returned')
      }

      // Parse the JSON result
      const decayStats = JSON.parse(result)
      console.log(`✅ Decay process completed - expired: ${decayStats.memories_expired}, compressed: ${decayStats.memories_compressed}`)

      return decayStats
    } catch (error) {
      console.error('❌ Error running decay process:', error)
      throw new Error(`Failed to run decay process: ${error.message}`)
    }
  }

  /**
     * Create a new session
     */
  async createSession (userId, name = null, metadata = {}) {
    this.ensureInitialized()

    try {
      // For now, we'll generate a session ID on the Node.js side
      // In a full implementation, this would call a Rust function
      const sessionId = this.generateSessionId()

      console.log(`📁 Created session ${sessionId} for user ${userId}`)
      return sessionId
    } catch (error) {
      console.error('❌ Error creating session:', error)
      throw new Error(`Failed to create session: ${error.message}`)
    }
  }

  /**
     * Get user sessions
     */
  async getUserSessions (userId) {
    this.ensureInitialized()

    try {
      // Get all memories for the user to reconstruct sessions
      const memories = await this.recallMemories({ userId, limit: 10000 })

      // Group memories by session
      const sessionMap = new Map()

      memories.forEach(memory => {
        if (!sessionMap.has(memory.sessionId)) {
          sessionMap.set(memory.sessionId, {
            id: memory.sessionId,
            userId: memory.userId,
            name: null, // Would be stored separately in full implementation
            createdAt: memory.timestamp,
            lastActive: memory.timestamp,
            memoryCount: 0,
            tags: [],
            metadata: {}
          })
        }

        const session = sessionMap.get(memory.sessionId)
        session.memoryCount++

        // Update last active time
        if (new Date(memory.timestamp) > new Date(session.lastActive)) {
          session.lastActive = memory.timestamp
        }

        // Update created time
        if (new Date(memory.timestamp) < new Date(session.createdAt)) {
          session.createdAt = memory.timestamp
        }
      })

      const sessions = Array.from(sessionMap.values())
      console.log(`✅ Found ${sessions.length} sessions for user ${userId}`)

      return sessions
    } catch (error) {
      console.error('❌ Error getting user sessions:', error)
      throw new Error(`Failed to get user sessions: ${error.message}`)
    }
  }

  /**
     * Get a specific session
     */
  async getSession (sessionId) {
    this.ensureInitialized()

    try {
      // Get memories for this session to reconstruct session info
      const memories = await this.recallMemories({ sessionId, limit: 1 })

      if (memories.length === 0) {
        return null
      }

      // Get all memories for the session to get accurate counts
      const allMemories = await this.recallMemories({
        userId: memories[0].userId,
        sessionId,
        limit: 10000
      })

      const session = {
        id: sessionId,
        userId: memories[0].userId,
        name: null, // Would be stored separately in full implementation
        createdAt: allMemories[allMemories.length - 1]?.timestamp,
        lastActive: allMemories[0]?.timestamp,
        memoryCount: allMemories.length,
        tags: [],
        metadata: {}
      }

      return session
    } catch (error) {
      console.error('❌ Error getting session:', error)
      throw new Error(`Failed to get session: ${error.message}`)
    }
  }

  /**
     * Get session memories
     */
  async getSessionMemories (userId, sessionId) {
    this.ensureInitialized()

    try {
      const memories = await this.recallMemories({ userId, sessionId, limit: 10000 })
      return memories
    } catch (error) {
      console.error('❌ Error getting session memories:', error)
      throw new Error(`Failed to get session memories: ${error.message}`)
    }
  }

  /**
     * Search sessions by keywords
     */
  async searchSessions (userId, keywords) {
    this.ensureInitialized()

    try {
      // Get all user sessions
      const sessions = await this.getUserSessions(userId)

      // For each session, check if any memories match the keywords
      const matchingSessions = []

      for (const session of sessions) {
        for (const keyword of keywords) {
          const memories = await this.recallMemories({
            userId,
            sessionId: session.id,
            query: keyword,
            limit: 1
          })

          if (memories.length > 0) {
            matchingSessions.push(session)
            break // Found a match, no need to check other keywords for this session
          }
        }
      }

      console.log(`✅ Found ${matchingSessions.length} sessions matching keywords`)
      return matchingSessions
    } catch (error) {
      console.error('❌ Error searching sessions:', error)
      throw new Error(`Failed to search sessions: ${error.message}`)
    }
  }

  /**
     * Export user memories
     */
  async exportUserMemories (userId) {
    this.ensureInitialized()

    try {
      const memories = await this.recallMemories({ userId, limit: 100000 })
      const exportData = JSON.stringify(memories, null, 2)

      console.log(`✅ Exported ${memories.length} memories for user ${userId}`)
      return exportData
    } catch (error) {
      console.error('❌ Error exporting memories:', error)
      throw new Error(`Failed to export memories: ${error.message}`)
    }
  }

  /**
     * Update session (placeholder implementation)
     */
  async updateSession (sessionId, updates) {
    // In a full implementation, this would update session metadata in Rust
    // For now, return a mock updated session
    const session = await this.getSession(sessionId)
    if (!session) {
      throw new Error('Session not found')
    }

    return {
      ...session,
      ...updates,
      updatedAt: new Date().toISOString()
    }
  }

  /**
     * Delete session (placeholder implementation)
     */
  async deleteSession (sessionId) {
    // In a full implementation, this would delete the session and its memories in Rust
    // For now, return mock deletion result
    const session = await this.getSession(sessionId)
    if (!session) {
      throw new Error('Session not found')
    }

    return {
      sessionId,
      memoriesDeleted: session.memoryCount || 0
    }
  }

  /**
     * Generate a unique session ID
     */
  generateSessionId () {
    return 'session_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9)
  }

  /**
     * Ensure the bridge is initialized
     */
  ensureInitialized () {
    if (!this.isInitialized || !this.cachePtr || this.cachePtr.isNull()) {
      throw new Error('Rust bridge not initialized. Call initialize() first.')
    }
  }

  /**
     * Cleanup resources
     */
  async cleanup () {
    try {
      if (this.cachePtr && !this.cachePtr.isNull()) {
        console.log('🧹 Cleaning up Rust bridge...')
        this.rustLib.mindcache_destroy(this.cachePtr)
        this.cachePtr = null
      }

      this.isInitialized = false
      console.log('✅ Rust bridge cleaned up')
    } catch (error) {
      console.error('❌ Error cleaning up Rust bridge:', error)
    }
  }

  /**
     * Health check
     */
  async healthCheck () {
    try {
      if (!this.isInitialized) {
        return { status: 'not_initialized' }
      }

      // Try to get stats as a health check
      await this.getStats()

      return {
        status: 'healthy',
        initialized: true,
        config: this.config
      }
    } catch (error) {
      return {
        status: 'unhealthy',
        error: error.message,
        initialized: this.isInitialized
      }
    }
  }
}

module.exports = RustBridge
