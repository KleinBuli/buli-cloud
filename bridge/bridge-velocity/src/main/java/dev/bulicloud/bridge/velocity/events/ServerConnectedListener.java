package dev.bulicloud.bridge.velocity.events;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.connection.DisconnectEvent;
import com.velocitypowered.api.event.player.ServerPostConnectEvent;
import dev.bulicloud.bridge.messaging.CloudPacket;
import dev.bulicloud.bridge.velocity.messaging.CloudNotificationService;

public final class ServerConnectedListener {

    private final CloudNotificationService notifications;

    public ServerConnectedListener(CloudNotificationService notifications) {
        this.notifications = notifications;
    }

    @Subscribe
    public void handlePlayerJoin(ServerPostConnectEvent event) {
        ObjectNode payload = notifications.playerPayload(event.getPlayer());
        event.getPlayer().getCurrentServer().ifPresent(connection ->
                payload.put("server", connection.getServerInfo().getName()));
        if (event.getPreviousServer() != null) {
            payload.put("previous_server", event.getPreviousServer().getServerInfo().getName());
        }
        notifications.sendPlayerNotification(
                event.getPreviousServer() == null ? CloudPacket.PLAYER_CONNECTED : CloudPacket.PLAYER_SWITCHED_SERVER,
                payload);
    }

    @Subscribe
    public void handlePlayerDisconnect(DisconnectEvent event) {
        if (event.getLoginStatus() == DisconnectEvent.LoginStatus.SUCCESSFUL_LOGIN) {
            notifications.sendPlayerNotification(CloudPacket.PLAYER_DISCONNECTED,
                    notifications.playerPayload(event.getPlayer()));
        }
    }
}
