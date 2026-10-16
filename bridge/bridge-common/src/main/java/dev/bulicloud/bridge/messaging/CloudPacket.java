package dev.bulicloud.bridge.messaging;

public enum CloudPacket {

    PLAYER_CONNECTED(PacketType.NOTIFICATION),
    PLAYER_DISCONNECTED(PacketType.NOTIFICATION),
    PLAYER_SWITCHED_SERVER(PacketType.NOTIFICATION),

    TRANSFER_PLAYER(PacketType.COMMAND),
    DISCONNECT_PLAYER(PacketType.COMMAND),

    SERVER_READY(PacketType.NOTIFICATION),
    SERVER_STOPPING(PacketType.NOTIFICATION),
    SERVER_STOPPED(PacketType.NOTIFICATION);

    private final PacketType packetType;
    CloudPacket(PacketType type) {
        this.packetType = type;
    }

}
