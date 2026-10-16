package dev.bulicloud.bridge.messaging;

import java.util.concurrent.CompletableFuture;

public interface MessageChannel {

    CompletableFuture<Void> send(CloudMessage message);

}
