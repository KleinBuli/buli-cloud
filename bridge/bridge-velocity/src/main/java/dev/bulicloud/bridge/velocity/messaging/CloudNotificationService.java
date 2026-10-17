package dev.bulicloud.bridge.velocity.messaging;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.velocitypowered.api.proxy.Player;
import dev.bulicloud.bridge.messaging.CloudMessage;
import dev.bulicloud.bridge.messaging.CloudPacket;
import dev.bulicloud.bridge.messaging.MessageChannel;
import org.slf4j.Logger;

import java.util.UUID;
import java.util.concurrent.CompletableFuture;

/** Builds and sends the proxy and player notifications emitted by this instance. */
public final class CloudNotificationService {

    private final MessageChannel channel;
    private final ObjectMapper objectMapper;
    private final String instanceId;
    private final Logger logger;

    public CloudNotificationService(MessageChannel channel, ObjectMapper objectMapper, String instanceId, Logger logger) {
        this.channel = channel;
        this.objectMapper = objectMapper;
        this.instanceId = instanceId;
        this.logger = logger;
    }

    public CompletableFuture<Void> sendProxyNotification(CloudPacket packet) {
        return send(packet, instancePayload());
    }

    public ObjectNode playerPayload(Player player) {
        ObjectNode payload = instancePayload();
        payload.put("uuid", player.getUniqueId().toString());
        payload.put("name", player.getUsername());
        payload.put("ipAddress", player.getRemoteAddress().getAddress().getHostAddress());
        return payload;
    }

    public void sendPlayerNotification(CloudPacket packet, ObjectNode payload) {
        send(packet, payload).exceptionally(error -> {
            logger.warn("Player notification failed", error);
            return null;
        });
    }

    private ObjectNode instancePayload() {
        ObjectNode payload = objectMapper.createObjectNode();
        payload.put("instance", instanceId);
        return payload;
    }

    private CompletableFuture<Void> send(CloudPacket packet, ObjectNode payload) {
        return channel.send(new CloudMessage(UUID.randomUUID().toString(), packet, payload));
    }
}
