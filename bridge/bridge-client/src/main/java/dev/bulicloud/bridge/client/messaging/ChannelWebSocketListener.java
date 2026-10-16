package dev.bulicloud.bridge.client.messaging;

import java.net.http.WebSocket;
import java.util.Objects;
import java.util.concurrent.CompletionStage;
import java.util.function.Consumer;

public class ChannelWebSocketListener implements WebSocket.Listener {

    private final Consumer<WebSocket> onDisconnect;
    public ChannelWebSocketListener(Consumer<WebSocket> onDisconnect) {
        this.onDisconnect = Objects.requireNonNull(onDisconnect);
    }

    @Override
    public CompletionStage<?> onClose(WebSocket webSocket, int statusCode, String reason) {
        onDisconnect.accept(webSocket);
        return WebSocket.Listener.super.onClose(webSocket, statusCode, reason);
    }

    @Override
    public void onError(WebSocket webSocket, Throwable error) {
        onDisconnect.accept(webSocket);
    }

}
