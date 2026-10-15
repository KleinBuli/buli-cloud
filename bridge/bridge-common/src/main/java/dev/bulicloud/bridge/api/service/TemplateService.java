package dev.bulicloud.bridge.api.service;

import dev.bulicloud.bridge.model.CloudTemplate;

import java.util.List;
import java.util.concurrent.CompletableFuture;

public interface TemplateService {

    CompletableFuture<Void> create(String groupName, String name);
    CompletableFuture<Void> delete(String groupName, String name);
    CompletableFuture<Void> get(String group, String template);
    CompletableFuture<List<CloudTemplate>> fromGroup(String name);
    CompletableFuture<Void> all();


}
