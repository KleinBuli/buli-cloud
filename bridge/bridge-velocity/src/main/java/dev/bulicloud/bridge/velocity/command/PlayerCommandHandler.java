package dev.bulicloud.bridge.velocity.command;

import com.fasterxml.jackson.databind.JsonNode;
import com.velocitypowered.api.proxy.ProxyServer;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;
import net.kyori.adventure.text.Component;
import org.slf4j.Logger;

import java.util.UUID;

public final class PlayerCommandHandler {

    private final ProxyServer server;
    private final InstanceRegister instances;
    private final Logger logger;

    public PlayerCommandHandler(ProxyServer server, InstanceRegister instances, Logger logger) {
        this.server = server;
        this.instances = instances;
        this.logger = logger;
    }

    public void transfer(JsonNode payload) {
        UUID uuid = UUID.fromString(payload.path("uuid").asText());
        String target = payload.path("server").asText();
        if (target.isBlank() || target.length() > 128) {
            throw new IllegalArgumentException("Invalid command payload");
        }
        disconnectReason(payload);
        if (payload.has("host")) {
            instances.registerServer(target, payload.path("host").asText(), payload.path("port").asInt());
        }
        server.getPlayer(uuid).ifPresent(player ->
                server.getServer(target).ifPresent(destination ->
                        player.createConnectionRequest(destination).connect()
                                .whenComplete((result, error) -> {
                                    if (error != null || result == null || !result.isSuccessful()) {
                                        logger.warn("Player transfer failed: {}", uuid);
                                    }
                                })));
    }

    public void disconnect(JsonNode payload) {
        UUID uuid = UUID.fromString(payload.path("uuid").asText());
        String reason = disconnectReason(payload);
        server.getPlayer(uuid).ifPresent(player -> player.disconnect(Component.text(reason)));
    }

    private String disconnectReason(JsonNode payload) {
        String reason = payload.path("reason").asText("Disconnected by the cloud");
        if (reason.length() > 4096) {
            throw new IllegalArgumentException("Invalid command payload");
        }
        return reason;
    }
}
