package dev.bulicloud.bridge.client.impl;

import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.api.service.CloudPlayerService;
import dev.bulicloud.bridge.api.service.GroupService;
import dev.bulicloud.bridge.api.service.InstanceService;
import dev.bulicloud.bridge.api.service.TemplateService;
import dev.bulicloud.bridge.client.CloudClient;

import java.util.Objects;

public class BuliCloudBridgeImpl implements BuliCloudBridge {

    private final CloudClient cloudClient;

    public BuliCloudBridgeImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }

    private CloudClient getCloudClient() {
        return cloudClient;
    }

    @Override
    public String getVersion() {
        return "0.1.0";
    }

    @Override
    public InstanceService instances() {
        return new InstanceServiceImpl(cloudClient);
    }

    @Override
    public TemplateService templates() {
        return new TemplateServiceImpl(cloudClient);
    }

    @Override
    public GroupService groups() {
        return new GroupServiceImpl(cloudClient);
    }

    @Override
    public CloudPlayerService cloudPlayers() {
        return new CloudPlayerServiceImpl(cloudClient);
    }
}
