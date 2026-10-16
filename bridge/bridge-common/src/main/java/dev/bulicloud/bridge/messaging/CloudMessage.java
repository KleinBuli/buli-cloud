package dev.bulicloud.bridge.messaging;

import com.fasterxml.jackson.databind.JsonNode;

public record CloudMessage(String id, CloudPacket packet, JsonNode payload) {
}
