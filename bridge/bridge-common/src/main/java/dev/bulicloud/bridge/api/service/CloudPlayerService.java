package dev.bulicloud.bridge.api.service;

import dev.bulicloud.bridge.model.CloudPlayer;
import java.util.List;
import java.util.concurrent.CompletableFuture;

public interface CloudPlayerService {
    CompletableFuture<CloudPlayer> get(String uuid);
    CompletableFuture<List<CloudPlayer>> all();
    CompletableFuture<Void> transfer(String uuid, String instance);
    CompletableFuture<Void> disconnect(String uuid, String reason);
}
