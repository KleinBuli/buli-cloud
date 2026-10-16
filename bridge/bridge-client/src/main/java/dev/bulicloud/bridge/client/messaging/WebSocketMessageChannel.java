package dev.bulicloud.bridge.client.messaging;

import com.fasterxml.jackson.databind.ObjectMapper;
import dev.bulicloud.bridge.messaging.CloudMessage;
import dev.bulicloud.bridge.messaging.MessageChannel;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.WebSocket;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public final class WebSocketMessageChannel implements MessageChannel {

    private final HttpClient client;
    private final URI endpoint;
    private final ObjectMapper mapper;
    private final String token;

    private WebSocket webSocket;
    private ChannelState state = ChannelState.DISCONNECTED;
    private ConnectionAttempt currentAttempt;

    private static final int MAX_PENDING = 256;

    private CompletableFuture<Void> sendTail = CompletableFuture.completedFuture(null);
    private int pending = 0;

    public WebSocketMessageChannel(URI endpoint, String token) {
        this.endpoint = Objects.requireNonNull(endpoint);
        this.token = Objects.requireNonNull(token);

        if (!"ws".equalsIgnoreCase(endpoint.getScheme()) && !"wss".equalsIgnoreCase(endpoint.getScheme())) {
            throw new IllegalArgumentException("Invalid WebSocket scheme");
        }

        if ("ws".equalsIgnoreCase(endpoint.getScheme()) && !("localhost".equalsIgnoreCase(endpoint.getHost()) || "127.0.0.1".equals(endpoint.getHost()) || "::1".equals(endpoint.getHost()))) {
            throw new IllegalArgumentException("Remote WebSocket requires WSS");
        }

        this.client = HttpClient.newHttpClient();
        this.mapper = new ObjectMapper();
    }

    public CompletableFuture<Void> connect() {
        ConnectionAttempt attempt;

        synchronized (this) {
            if (state != ChannelState.DISCONNECTED) {
                return CompletableFuture.failedFuture(new IllegalStateException("Channel already connecting or connected"));
            }

            state = ChannelState.CONNECTING;
            attempt = new ConnectionAttempt();
            currentAttempt = attempt;
        }

        CompletableFuture<WebSocket> connection;

        try {
            connection = client.newWebSocketBuilder().header("Authorization", "Bearer " + token).buildAsync(endpoint, new ChannelWebSocketListener(ws -> handleDisconnect(attempt, ws)));
        } catch (RuntimeException error) {
            failConnection(attempt);
            return CompletableFuture.failedFuture(error);
        }

        return connection.thenAccept(ws -> {
            boolean accepted;

            synchronized (this) {
                accepted = currentAttempt == attempt && state == ChannelState.CONNECTING && !attempt.disconnected;

                if (accepted) {
                    webSocket = ws;
                    state = ChannelState.CONNECTED;
                }
            }

            if (!accepted) {
                ws.abort();
                throw new IllegalStateException("WebSocket closed during connection");
            }
        }).whenComplete((result, error) -> {
            if (error != null) {
                failConnection(attempt);
            }
        });
    }

    public CompletableFuture<Void> disconnect() {
        WebSocket ws;

        synchronized (this) {
            if (state == ChannelState.DISCONNECTED) {
                return CompletableFuture.completedFuture(null);
            }

            if (state == ChannelState.CONNECTING) {
                return CompletableFuture.failedFuture(
                        new IllegalStateException("Channel is still connecting")
                );
            }

            ws = webSocket;
            webSocket = null;
            state = ChannelState.DISCONNECTED;
            currentAttempt = null;
        }

        if (ws == null) {
            return CompletableFuture.completedFuture(null);
        }

        return ws.sendClose(
                WebSocket.NORMAL_CLOSURE,
                "Bridge shutting down"
        ).thenApply(ignored -> (Void) null);
    }

    @Override
    public CompletableFuture<Void> send(CloudMessage message) {
        Objects.requireNonNull(message);

        final String json;

        try {
            json = mapper.writeValueAsString(message);
        } catch (Exception error) {
            return CompletableFuture.failedFuture(error);
        }

        synchronized (this) {
            if (state != ChannelState.CONNECTED || webSocket == null) {
                return CompletableFuture.failedFuture(
                        new IllegalStateException("MessageChannel is not connected")
                );
            }

            if (pending >= MAX_PENDING) {
                return CompletableFuture.failedFuture(
                        new IllegalStateException("Message queue is full")
                );
            }

            WebSocket ws = webSocket;
            pending++;

            CompletableFuture<Void> operation = sendTail.thenCompose(ignored -> {
                synchronized (this) {
                    if (state != ChannelState.CONNECTED || webSocket != ws) {
                        return CompletableFuture.failedFuture(
                                new IllegalStateException("Connection was closed")
                        );
                    }
                }

                return ws.sendText(json, true)
                        .thenApply(ignoredSocket -> (Void) null);
            });

            sendTail = operation.handle((result, error) -> null);

            return operation.whenComplete((result, error) -> {
                synchronized (this) {
                    pending--;
                }
            });
        }
    }


    private synchronized void handleDisconnect(ConnectionAttempt attempt, WebSocket ws) {
        if (currentAttempt != attempt) {
            return;
        }

        attempt.disconnected = true;

        if (webSocket != null && webSocket != ws) {
            return;
        }

        webSocket = null;
        state = ChannelState.DISCONNECTED;
        currentAttempt = null;
    }

    private synchronized void failConnection(ConnectionAttempt attempt) {
        if (currentAttempt != attempt) {
            return;
        }

        webSocket = null;
        state = ChannelState.DISCONNECTED;
        currentAttempt = null;
    }

    public synchronized ChannelState state() {
        return state;
    }

    private static final class ConnectionAttempt {
        private boolean disconnected;
    }
}
