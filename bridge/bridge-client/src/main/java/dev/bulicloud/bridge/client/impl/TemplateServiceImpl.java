package dev.bulicloud.bridge.client.impl;

import com.fasterxml.jackson.core.type.TypeReference;
import dev.bulicloud.bridge.api.service.TemplateService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudTemplate;

import java.util.List;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public class TemplateServiceImpl implements TemplateService {

    private final CloudClient cloudClient;

    public TemplateServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }


    @Override
    public CompletableFuture<Void> create(String groupName, String name) {
        var path = String.format("templates/%s/%s", groupName, name);
        var request  = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> delete(String groupName, String name) {
        var path = String.format("templates/%s/%s", groupName, name);
        var request  = cloudClient.delete(path);

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }


    @Override
    public CompletableFuture<CloudTemplate> get(String group, String template) {
        var path = String.format("templates/%s/%s", group, template);
        var request  = cloudClient.get(path, new TypeReference<CloudTemplate>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<List<CloudTemplate>> all() {
        var path = "templates";
        var request  = cloudClient.get(path, new TypeReference<List<String>>() {}).thenCompose(names -> {
            var requests = names.stream().map(name -> {
                var parts = name.split("/", 2);
                return get(parts[0], parts[1]);
            }).toList();
            return CompletableFuture.allOf(requests.toArray(CompletableFuture[]::new))
                    .thenApply(ignored -> requests.stream().map(CompletableFuture::join).toList());
        });

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }
}
