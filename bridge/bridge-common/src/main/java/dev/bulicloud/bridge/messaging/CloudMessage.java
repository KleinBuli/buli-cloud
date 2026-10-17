package dev.bulicloud.bridge.messaging;

import com.fasterxml.jackson.databind.JsonNode;

public record CloudMessage(String id, CloudPacket packet, JsonNode payload) {
    public CloudMessage {
        if (id == null || id.isBlank() || id.length() > 128 || packet == null || payload == null || !payload.isObject()) {
            throw new IllegalArgumentException("Invalid cloud message");
        }
    }
}
