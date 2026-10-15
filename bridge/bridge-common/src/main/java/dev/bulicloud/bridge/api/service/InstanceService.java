package dev.bulicloud.bridge.api.service;

import dev.bulicloud.bridge.model.CloudInstance;

import java.util.List;
import java.util.concurrent.CompletableFuture;

public interface InstanceService {

    CompletableFuture<Void> create(String group, String cloudTemplate);
    CompletableFuture<Void> start(String id);
    CompletableFuture<Void> stop(String id);
    CompletableFuture<CloudInstance> get(String id);
    CompletableFuture<List<CloudInstance>> all();

}
