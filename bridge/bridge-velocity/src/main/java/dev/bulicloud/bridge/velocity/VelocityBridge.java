package dev.bulicloud.bridge.velocity;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.inject.Inject;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.messaging.CloudMessage;
import dev.bulicloud.bridge.messaging.CloudPacket;

import java.util.UUID;
import java.util.logging.Logger;

@Plugin(
        id = "bulicloud",
        name = "BuliCloud",
        version = "0.1.0",
        authors = {"KleinBuli"}
)
public final class VelocityBridge {

    private final ProxyServer server;
    private final Logger logger;

    private CloudClient cloudClient;
    private ObjectMapper objectMapper;

    private static VelocityBridge instance;

    @Inject
    public VelocityBridge(ProxyServer server, Logger logger) {
        this.server = server;
        this.logger = logger;
    }

    @Subscribe
    public void onInitialize(ProxyInitializeEvent event) {
        logger.info("[BuliCloud Velocity Bridge] velocity bridge initialized");

        instance = this;

        String token = System.getenv("BRIDGE_TOKEN");
        this.cloudClient = new CloudClient("http://127.0.0.1:8080/", token);
        this.objectMapper = new ObjectMapper();
    }

    private void sendProxyReadyNotification() {
        ObjectNode payload = VelocityBridge.getInstance().getObjectMapper().createObjectNode();
        payload.put("instance", getServer().get)

        CloudMessage cloudMessage = new CloudMessage(
                UUID.randomUUID().toString(),
                CloudPacket.SERVER_READY,
                payload
        );

        VelocityBridge.getInstance().getCloudClient().messageChannel().send(cloudMessage);
    }

    public ProxyServer getServer() {
        return server;
    }

    public CloudClient getCloudClient() {
        return cloudClient;
    }

    public static VelocityBridge getInstance() {
        return instance;
    }

    public ObjectMapper getObjectMapper() {
        return objectMapper;
    }
}