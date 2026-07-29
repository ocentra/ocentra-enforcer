// Realtime chat + notification backend. Origin-allowlisted equivalent of
// the vuln.js fixture: every WebSocket surface validates the handshake's
// Origin against an explicit allowlist before accepting a connection.
const http = require('http');
const { Server } = require('socket.io');
const WebSocket = require('ws');

const httpServer = http.createServer();

const ALLOWED_ORIGINS = ['https://app.example.com', 'https://admin.example.com'];

function isAllowedOrigin(origin) {
  return ALLOWED_ORIGINS.includes(origin);
}

// socket.io CORS is scoped to the explicit allowlist above, not a wildcard.
const io = new Server(httpServer, {
  cors: {
    origin: ALLOWED_ORIGINS,
    methods: ['GET', 'POST'],
  },
});

io.on('connection', (socket) => {
  socket.on('get_messages', (channelId) => {
    socket.emit('messages', loadPrivateMessages(socket.request, channelId));
  });
});

// Raw ws server for the trading feed: verifyClient validates the Origin
// header against the allowlist before ever accepting the handshake.
const wss = new WebSocket.Server({
  port: 8080,
  verifyClient: (info, cb) => {
    const origin = info.origin || info.req.headers.origin;
    if (isAllowedOrigin(origin)) {
      cb(true);
    } else {
      cb(false);
    }
  },
});

wss.on('connection', (ws) => {
  ws.on('message', (data) => broadcastToAllSubscribers(data));
});

// Internal health-check socket: no verifyClient/checkOrigin key at all.
// Mere absence of a hook is not itself a misconfiguration.
const internalWss = new WebSocket.Server({ port: 8081 });

// Client-side connection to a trusted upstream market-data feed — this is
// not server configuration and is out of this rule's scope.
const upstreamFeed = new WebSocket('wss://upstream.example.com/feed');

function loadPrivateMessages(req, channelId) {
  return db.messages.findByChannel(channelId, req.headers.cookie);
}

function broadcastToAllSubscribers(data) {
  wss.clients.forEach((client) => client.send(data));
}

module.exports = { io, wss, internalWss, upstreamFeed };
