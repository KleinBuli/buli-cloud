package dev.bulicloud.bridge.client.impl;

import com.fasterxml.jackson.core.type.TypeReference;
import dev.bulicloud.bridge.api.service.CloudPlayerService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudPlayer;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;

public final class CloudPlayerServiceImpl implements CloudPlayerService {
    private final CloudClient cloudClient;

    public CloudPlayerServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }

    @Override
    public CompletableFuture<CloudPlayer> get(String uuid) {
        return cloudClient.get(path(uuid), new TypeReference<CloudPlayer>() {});
    }

    @Override
    public CompletableFuture<List<CloudPlayer>> all() {
        return cloudClient.get("players", new TypeReference<List<CloudPlayer>>() {});
    }

    @Override
    public CompletableFuture<Void> transfer(String uuid, String instance) {
        return cloudClient.post(path(uuid) + "/transfer", Map.of("server", Objects.requireNonNull(instance)), new TypeReference<Void>() {});
    }

    @Override
    public CompletableFuture<Void> disconnect(String uuid, String reason) {
        return cloudClient.post(path(uuid) + "/disconnect", Map.of("reason", Objects.requireNonNull(reason)), new TypeReference<Void>() {});
    }

    private String path(String uuid) {
        return "players/" + UUID.fromString(Objects.requireNonNull(uuid));
    }
}
