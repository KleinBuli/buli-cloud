package dev.bulicloud.bridge.velocity.command;

import com.fasterxml.jackson.databind.JsonNode;
import dev.bulicloud.bridge.velocity.VelocityBridge;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;

public final class ServerCommandHandler {

    private final InstanceRegister instances;

    public ServerCommandHandler(InstanceRegister instances) {
        this.instances = instances;
    }

    public void register(JsonNode payload) {
        var name = payload.path("name").asText();
        var group = payload.path("group").asText(name);
        instances.registerServer(name, payload.path("host").asText(), payload.path("port").asInt(), group);

        var bridge = VelocityBridge.getInstance();
        var fallbackManager = bridge.getRuntime().getFallbackManager();
        bridge.getBridge().groups().fallback().thenAccept(fallback -> {
            instances.setFallbackGroups(fallback);
            if (fallback.stream().anyMatch(fb -> fb.equalsIgnoreCase(group))) {
                fallbackManager.addFallback(name);
            } else {
                fallbackManager.removeFallback(name);
            }
        });
    }

    public void unregister(JsonNode payload) {
        var name = payload.path("name").asText();
        instances.unregisterServer(name);
        VelocityBridge.getInstance().getRuntime().getFallbackManager().removeFallback(name);
    }
}