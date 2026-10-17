package dev.bulicloud.bridge.client.messaging;

import java.net.http.WebSocket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Objects;
import java.util.concurrent.CompletionStage;
import java.util.function.Consumer;

public class ChannelWebSocketListener implements WebSocket.Listener {

    private final Consumer<WebSocket> onDisconnect;
    private final Consumer<String> onMessage;
    private final StringBuilder text = new StringBuilder();

    public ChannelWebSocketListener(Consumer<WebSocket> onDisconnect, Consumer<String> onMessage) {
        this.onDisconnect = Objects.requireNonNull(onDisconnect);
        this.onMessage = Objects.requireNonNull(onMessage);
    }

    @Override
    public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
        if (text.length() + data.length() > 64 * 1024) {
            fail(webSocket);
            return null;
        }
        text.append(data);
        if (last) {
            String message = text.toString();
            text.setLength(0);
            try {
                if (message.getBytes(StandardCharsets.UTF_8).length > 64 * 1024) {
                    throw new IllegalArgumentException("Bridge message is too large");
                }
                onMessage.accept(message);
            } catch (RuntimeException error) {
                fail(webSocket);
                return null;
            }
        }
        webSocket.request(1);
        return null;
    }

    @Override
    public CompletionStage<?> onBinary(WebSocket webSocket, ByteBuffer data, boolean last) {
        fail(webSocket);
        return null;
    }

    private void fail(WebSocket webSocket) {
        text.setLength(0);
        webSocket.abort();
        onDisconnect.accept(webSocket);
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
