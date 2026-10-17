package dev.bulicloud.bridge.velocity.command;

import com.fasterxml.jackson.databind.JsonNode;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;

public final class ServerCommandHandler {

    private final InstanceRegister instances;

    public ServerCommandHandler(InstanceRegister instances) {
        this.instances = instances;
    }

    public void register(JsonNode payload) {
        String name = payload.path("name").asText();
        instances.registerServer(name, payload.path("host").asText(), payload.path("port").asInt(),
                payload.path("group").asText(name));
    }

    public void unregister(JsonNode payload) {
        instances.unregisterServer(payload.path("name").asText());
    }
}
