// Realtime chat + notification backend. Insecure WebSocket configuration:
// three separate CSWSH-enabling misconfigurations live here.
const http = require('http');
const { Server } = require('socket.io');
const WebSocket = require('ws');

const httpServer = http.createServer();

// (1) socket.io permissive CORS: any origin can open an authenticated
// WebSocket connection and read private chat/notification traffic.
const io = new Server(httpServer, {
  cors: {
    origin: '*',
    methods: ['GET', 'POST'],
  },
});

io.on('connection', (socket) => {
  socket.on('get_messages', (channelId) => {
    socket.emit('messages', loadPrivateMessages(socket.request, channelId));
  });
});

// (2) Legacy socket.io v2 namespace kept around for an old mobile client —
// still wide open to every origin and every port.
const legacyIo = require('socket.io')(httpServer);
legacyIo.origins('*:*');

// (3) Raw ws server for the trading feed: verifyClient always accepts the
// handshake, so the Origin header is never actually inspected.
const wss = new WebSocket.Server({
  port: 8080,
  verifyClient: (info, cb) => cb(true),
});

wss.on('connection', (ws) => {
  ws.on('message', (data) => broadcastToAllSubscribers(data));
});

function loadPrivateMessages(req, channelId) {
  return db.messages.findByChannel(channelId, req.headers.cookie);
}

function broadcastToAllSubscribers(data) {
  wss.clients.forEach((client) => client.send(data));
}

module.exports = { io, legacyIo, wss };
