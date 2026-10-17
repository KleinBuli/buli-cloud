package dev.bulicloud.bridge.velocity;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.inject.Inject;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.player.PlayerChooseInitialServerEvent;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.event.proxy.ProxyShutdownEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.client.impl.BuliCloudBridgeImpl;
import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.api.BuliCloudProvider;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;
import dev.bulicloud.bridge.messaging.CloudMessage;
import dev.bulicloud.bridge.messaging.CloudPacket;

import java.util.UUID;
import java.util.Locale;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import dev.bulicloud.bridge.velocity.events.ServerConnectedListener;
import net.kyori.adventure.text.Component;
import org.slf4j.Logger;

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
    private InstanceRegister instanceRegister;
    private BuliCloudBridge bridge;

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
        this.instanceRegister = new InstanceRegister(server);
        this.bridge = new BuliCloudBridgeImpl(cloudClient);
        BuliCloudProvider.register(bridge);

        cloudClient.messageChannel().onMessage(this::handleCommand);
        server.getEventManager().register(this, new ServerConnectedListener());
        cloudClient.messageChannel().connect()
                .thenCompose(ignored -> sendProxyNotification(CloudPacket.SERVER_READY))
                .exceptionally(error -> {
                    logger.error("Bridge initialization failed: " + error.getMessage());
                    return null;
                });
    }

    @Subscribe
    public void onChooseInitialServer(PlayerChooseInitialServerEvent event) {
        if (instanceRegister == null) return;
        var config = server.getConfiguration();
        var targets = event.getPlayer().getVirtualHost()
                .map(host -> config.getForcedHosts().get(host.getHostString().toLowerCase(Locale.ROOT)))
                .orElse(config.getAttemptConnectionOrder());
        for (String target : targets) {
            var destination = instanceRegister.findServer(target);
            if (destination.isPresent()) {
                event.setInitialServer(destination.get());
                return;
            }
        }
    }

    @Subscribe
    public void onStop(ProxyShutdownEvent event) {
        if (cloudClient == null) return;
        try {
            sendProxyNotification(CloudPacket.SERVER_STOPPING)
                    .handle((result, error) -> null)
                    .thenCompose(ignored -> cloudClient.messageChannel().disconnect())
                    .get(6, TimeUnit.SECONDS);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        } catch (Exception error) {
            logger.warn("Bridge shutdown failed: " + error.getMessage());
        } finally {
            if (bridge != null) BuliCloudProvider.unregister(bridge);
        }
    }

    private CompletableFuture<Void> sendProxyNotification(CloudPacket packet) {

        var serverName = System.getenv("INSTANCE_ID");
        ObjectNode payload = VelocityBridge.getInstance().getObjectMapper().createObjectNode();
        payload.put("instance", serverName);

        CloudMessage cloudMessage = new CloudMessage(
                UUID.randomUUID().toString(),
                packet,
                payload
        );

        return cloudClient.messageChannel().send(cloudMessage);
    }

    private void handleCommand(CloudMessage message) {
        if (message.packet() == CloudPacket.REGISTER_SERVER) {
            instanceRegister.registerServer(message.payload().path("name").asText(),
                    message.payload().path("host").asText(), message.payload().path("port").asInt(),
                    message.payload().path("group").asText(message.payload().path("name").asText()));
            return;
        }
        if (message.packet() == CloudPacket.UNREGISTER_SERVER) {
            instanceRegister.unregisterServer(message.payload().path("name").asText());
            return;
        }
        if (message.packet() != CloudPacket.TRANSFER_PLAYER && message.packet() != CloudPacket.DISCONNECT_PLAYER) {
            throw new IllegalArgumentException("Unexpected bridge packet");
        }
        UUID uuid = UUID.fromString(message.payload().path("uuid").asText());
        String target = message.payload().path("server").asText();
        String reason = message.payload().path("reason").asText("Disconnected by the cloud");
        if (message.packet() == CloudPacket.TRANSFER_PLAYER && (target.isBlank() || target.length() > 128)
                || reason.length() > 4096) {
            throw new IllegalArgumentException("Invalid command payload");
        }
        if (message.packet() == CloudPacket.TRANSFER_PLAYER && message.payload().has("host")) {
            instanceRegister.registerServer(target, message.payload().path("host").asText(), message.payload().path("port").asInt());
        }
        server.getPlayer(uuid).ifPresent(player -> {
            if (message.packet() == CloudPacket.TRANSFER_PLAYER) {
                server.getServer(target).ifPresent(destination -> player.createConnectionRequest(destination).connect()
                        .whenComplete((result, error) -> {
                            if (error != null || result == null || !result.isSuccessful()) logger.warn("Player transfer failed: " + uuid);
                        }));
            } else {
                player.disconnect(Component.text(reason));
            }
        });
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