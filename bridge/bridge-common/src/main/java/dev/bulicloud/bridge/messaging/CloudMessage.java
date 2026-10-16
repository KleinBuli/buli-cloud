package dev.bulicloud.bridge.messaging;

import com.fasterxml.jackson.databind.JsonNode;

public record CloudMessage(CloudPacket packet, JsonNode payload, String requestId) {
}
