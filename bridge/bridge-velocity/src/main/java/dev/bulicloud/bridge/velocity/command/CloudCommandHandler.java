package dev.bulicloud.bridge.velocity.command;

import dev.bulicloud.bridge.messaging.CloudMessage;

/** Routes incoming cloud packets to the responsible command handler. */
public final class CloudCommandHandler {

    private final ServerCommandHandler servers;
    private final PlayerCommandHandler players;

    public CloudCommandHandler(ServerCommandHandler servers, PlayerCommandHandler players) {
        this.servers = servers;
        this.players = players;
    }

    public void handle(CloudMessage message) {
        switch (message.packet()) {
            case REGISTER_SERVER -> servers.register(message.payload());
            case UNREGISTER_SERVER -> servers.unregister(message.payload());
            case TRANSFER_PLAYER -> players.transfer(message.payload());
            case DISCONNECT_PLAYER -> players.disconnect(message.payload());
            default -> throw new IllegalArgumentException("Unexpected bridge packet");
        }
    }
}
