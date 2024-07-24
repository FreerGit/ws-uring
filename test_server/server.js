var port = 8080,
  WebSocketServer = require("ws").Server,
  wss = new WebSocketServer({ port: port });

console.log("listening on port: " + port);

wss.on("connection", function connection(ws) {
  console.log("new client connected!");

  ws.on("message", function (message) {
    console.log("message: " + message);
    ws.send("echo: " + message);
  });
});
