package dev.bulicloud.bridge.api;

import dev.bulicloud.bridge.api.service.CloudPlayerService;
import dev.bulicloud.bridge.api.service.GroupService;
import dev.bulicloud.bridge.api.service.InstanceService;
import dev.bulicloud.bridge.api.service.TemplateService;

public interface BuliCloudBridge {

    String getVersion();
    InstanceService instances();
    TemplateService templates();
    GroupService groups();
    CloudPlayerService cloudPlayers();

}