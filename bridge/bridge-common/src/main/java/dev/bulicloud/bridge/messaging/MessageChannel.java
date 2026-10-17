package dev.bulicloud.bridge.messaging;

import java.util.concurrent.CompletableFuture;
import java.util.function.Consumer;

public interface MessageChannel {

    CompletableFuture<Void> connect();

    CompletableFuture<Void> disconnect();

    void onMessage(Consumer<CloudMessage> handler);

    CompletableFuture<Void> send(CloudMessage message);

}
