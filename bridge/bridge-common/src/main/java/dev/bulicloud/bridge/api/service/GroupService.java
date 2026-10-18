package dev.bulicloud.bridge.api.service;

import dev.bulicloud.bridge.model.CloudGroup;

import java.util.List;
import java.util.concurrent.CompletableFuture;

public interface GroupService {

    default CompletableFuture<Void> create(String name) {
        return create(name, dev.bulicloud.bridge.model.enums.GroupRole.PROXIED_SERVER);
    }
    CompletableFuture<Void> create(String name, dev.bulicloud.bridge.model.enums.GroupRole role);
    CompletableFuture<Void> delete(String name);
    CompletableFuture<Void> setMaintenance(String name, boolean maintenance);
    CompletableFuture<CloudGroup> get(String name);
    CompletableFuture<List<String>> fallback();
    CompletableFuture<List<CloudGroup>> all();

}
