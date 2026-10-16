package dev.bulicloud.bridge.velocity.events;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.player.ServerConnectedEvent;
import com.velocitypowered.api.proxy.Player;
import dev.bulicloud.bridge.messaging.CloudMessage;
import dev.bulicloud.bridge.messaging.CloudPacket;
import dev.bulicloud.bridge.velocity.VelocityBridge;

import java.util.UUID;

public class ServerConnectedListener {

    @Subscribe
    public void handlePlayerJoin(ServerConnectedEvent event) {
        Player player = event.getPlayer();
        ObjectNode payload = VelocityBridge.getInstance().getObjectMapper().createObjectNode();
        payload.put("uuid", player.getUniqueId().toString());
        payload.put("name", player.getUsername());

        CloudMessage cloudMessage = new CloudMessage(
                UUID.randomUUID().toString(),
                CloudPacket.PLAYER_CONNECTED,
                payload
        );

        VelocityBridge.getInstance().getCloudClient().messageChannel().send(cloudMessage);

    }

}
