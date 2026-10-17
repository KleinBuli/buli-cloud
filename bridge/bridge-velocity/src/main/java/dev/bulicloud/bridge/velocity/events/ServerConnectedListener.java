package dev.bulicloud.bridge.velocity.events;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.player.ServerPostConnectEvent;
import com.velocitypowered.api.event.connection.DisconnectEvent;
import com.velocitypowered.api.proxy.Player;
import dev.bulicloud.bridge.messaging.CloudMessage;
import dev.bulicloud.bridge.messaging.CloudPacket;
import dev.bulicloud.bridge.velocity.VelocityBridge;

import java.util.UUID;

public class ServerConnectedListener {

    @Subscribe
    public void handlePlayerJoin(ServerPostConnectEvent event) {
        ObjectNode payload = playerPayload(event.getPlayer());
        event.getPlayer().getCurrentServer().ifPresent(connection ->
                payload.put("server", connection.getServerInfo().getName()));
        if (event.getPreviousServer() != null) {
            payload.put("previous_server", event.getPreviousServer().getServerInfo().getName());
        }
        send(event.getPreviousServer() == null ? CloudPacket.PLAYER_CONNECTED : CloudPacket.PLAYER_SWITCHED_SERVER, payload);
    }

    @Subscribe
    public void handlePlayerDisconnect(DisconnectEvent event) {
        if (event.getLoginStatus() == DisconnectEvent.LoginStatus.SUCCESSFUL_LOGIN) {
            send(CloudPacket.PLAYER_DISCONNECTED, playerPayload(event.getPlayer()));
        }
    }

    private ObjectNode playerPayload(Player player) {
        ObjectNode payload = VelocityBridge.getInstance().getObjectMapper().createObjectNode();
        payload.put("instance", System.getenv("INSTANCE_ID"));
        payload.put("uuid", player.getUniqueId().toString());
        payload.put("name", player.getUsername());
        payload.put("ipAddress", player.getRemoteAddress().getAddress().getHostAddress());
        return payload;
    }

    private void send(CloudPacket packet, ObjectNode payload) {
        VelocityBridge.getInstance().getCloudClient().messageChannel()
                .send(new CloudMessage(UUID.randomUUID().toString(), packet, payload))
                .exceptionally(error -> {
                    System.err.println("[Bridge] Player notification failed: " + error.getMessage());
                    return null;
                });
    }
}
