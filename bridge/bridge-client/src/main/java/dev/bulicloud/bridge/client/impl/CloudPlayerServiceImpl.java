package dev.bulicloud.bridge.client.impl;

import dev.bulicloud.bridge.api.service.CloudPlayerService;
import dev.bulicloud.bridge.client.CloudClient;

import java.util.Objects;

public class CloudPlayerServiceImpl implements CloudPlayerService {

    private final CloudClient cloudClient;

    public CloudPlayerServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }


}
