package dev.bulicloud.bridge.velocity.command;

import com.fasterxml.jackson.databind.JsonNode;
import dev.bulicloud.bridge.velocity.VelocityBridge;
import dev.bulicloud.bridge.velocity.fallback.FallbackManager;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;

public final class ServerCommandHandler {

    private final InstanceRegister instances;

    public ServerCommandHandler(InstanceRegister instances) {
        this.instances = instances;
    }

    public void register(JsonNode payload) {
        var name = payload.path("name").asText();

        instances.registerServer(name, payload.path("host").asText(), payload.path("port").asInt(),
                payload.path("group").asText(name));

        var fallback_manager = VelocityBridge.getInstance().getRuntime().getFallbackManager();
        VelocityBridge.getInstance().getBridge().groups().fallback().thenAccept(
                fallback -> fallback.stream().filter(fb -> fb.equalsIgnoreCase(name)).forEach(fallback_manager::addFallback));

    }

    public void unregister(JsonNode payload) {

        var name = payload.path("name").asText();
        instances.unregisterServer(payload.path(name).asText());

        var fallback_manager = VelocityBridge.getInstance().getRuntime().getFallbackManager();
        VelocityBridge.getInstance().getBridge().groups().fallback().thenAccept(
                fallback -> fallback.stream().filter(fb -> fb.equalsIgnoreCase(name)).forEach(fallback_manager::addFallback));

    }
}
