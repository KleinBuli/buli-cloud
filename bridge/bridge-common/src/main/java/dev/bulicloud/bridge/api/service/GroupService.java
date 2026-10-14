package dev.bulicloud.bridge.api.service;

import dev.bulicloud.bridge.model.CloudGroup;

import java.util.List;
import java.util.concurrent.CompletableFuture;

public interface GroupService {

    CompletableFuture<Void> create(String name);
    CompletableFuture<CloudGroup> get(String name);
    CompletableFuture<List<CloudGroup>> all();

}
