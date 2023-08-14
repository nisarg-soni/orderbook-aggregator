
const WebSocket = require('ws');
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');
const express = require('express')
const http = require('http');
const path = require('path');

const app = express()



const PROTO_PATH = '../protos/orderbook.proto';

const packageDefinition = protoLoader.loadSync(
    PROTO_PATH, {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
});

const summaryProto = grpc.loadPackageDefinition(packageDefinition).orderbook;

function main() {
    const client = new summaryProto.OrderbookAggregator('127.0.0.1:7050', grpc.credentials.createInsecure());

    app.use(express.static(path.join(__dirname, 'public')));

    const server = http.createServer(app);
    const wss = new WebSocket.Server({ server });

    wss.on('connection', (ws) => {
        console.log('WebSocket connection established');
        ws.on('open', () => {
            let call = client.BookSummary();

            call.on('data', function (book) {
                ws.send(JSON.stringify(book));
            });
        });
        ws.on('message', () => {
            let call = client.BookSummary();

            call.on('data', function (book) {
                ws.send(JSON.stringify(book));
            });
        });

        let call = client.BookSummary();

        call.on('data', function (book) {
            ws.send(JSON.stringify(book));
        });
    });



    server.listen(7000, () => {
        console.log('Server listening on port 7000');
    });

}

main();