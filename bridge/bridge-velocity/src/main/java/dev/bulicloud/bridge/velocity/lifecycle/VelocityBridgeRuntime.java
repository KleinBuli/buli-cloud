package dev.bulicloud.bridge.velocity.lifecycle;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.velocitypowered.api.proxy.ProxyServer;
import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.api.BuliCloudProvider;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.client.impl.BuliCloudBridgeImpl;
import dev.bulicloud.bridge.messaging.CloudPacket;
import dev.bulicloud.bridge.messaging.MessageChannel;
import dev.bulicloud.bridge.velocity.command.CloudCommandHandler;
import dev.bulicloud.bridge.velocity.command.PlayerCommandHandler;
import dev.bulicloud.bridge.velocity.command.ServerCommandHandler;
import dev.bulicloud.bridge.velocity.events.InitialServerListener;
import dev.bulicloud.bridge.velocity.events.ServerConnectedListener;
import dev.bulicloud.bridge.velocity.fallback.FallbackManager;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;
import dev.bulicloud.bridge.velocity.messaging.CloudNotificationService;
import org.slf4j.Logger;

import java.util.concurrent.TimeUnit;

/** Owns the bridge components and their startup/shutdown order. */
public final class VelocityBridgeRuntime {

    private final Object plugin;
    private final ProxyServer server;
    private final Logger logger;
    private final MessageChannel channel;
    private final BuliCloudBridge bridge;
    private final CloudNotificationService notifications;
    private final CloudCommandHandler commands;
    private final InitialServerListener initialServerListener;
    private final ServerConnectedListener serverConnectedListener;
    private final FallbackManager fallbackManager;

    public VelocityBridgeRuntime(Object plugin, ProxyServer server, Logger logger) {
        this.plugin = plugin;
        this.server = server;
        this.logger = logger;

        CloudClient client = new CloudClient("http://127.0.0.1:8080/", System.getenv("BRIDGE_TOKEN"));
        InstanceRegister instances = new InstanceRegister(server);
        channel = client.messageChannel();
        bridge = new BuliCloudBridgeImpl(client);
        notifications = new CloudNotificationService(channel, new ObjectMapper(), System.getenv("INSTANCE_ID"), logger);
        commands = new CloudCommandHandler(
                new ServerCommandHandler(instances), new PlayerCommandHandler(server, instances, logger));
        initialServerListener = new InitialServerListener(server, instances);
        serverConnectedListener = new ServerConnectedListener(notifications);
        fallbackManager = new FallbackManager(server);

    }

    public void start() {
        BuliCloudProvider.register(bridge);
        server.getCommandManager().register(server.getCommandManager().metaBuilder("bulicloud").plugin(plugin).build(),
                new dev.bulicloud.bridge.velocity.command.ingame.CloudCommand(bridge, logger));
        channel.onMessage(commands::handle);
        server.getEventManager().register(plugin, initialServerListener);
        server.getEventManager().register(plugin, serverConnectedListener);
        channel.connect()
                .thenCompose(ignored -> notifications.sendProxyNotification(CloudPacket.SERVER_READY))
                .exceptionally(error -> {
                    logger.error("Bridge initialization failed", error);
                    return null;
                });
        logger.info("[BuliCloud Velocity Bridge] velocity bridge initialized");
    }

    public void stop() {
        server.getCommandManager().unregister("bulicloud");
        try {
            notifications.sendProxyNotification(CloudPacket.SERVER_STOPPING)
                    .handle((result, error) -> null)
                    .thenCompose(ignored -> channel.disconnect())
                    .get(6, TimeUnit.SECONDS);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        } catch (Exception error) {
            logger.warn("Bridge shutdown failed", error);
        } finally {
            server.getEventManager().unregisterListener(plugin, initialServerListener);
            server.getEventManager().unregisterListener(plugin, serverConnectedListener);
            BuliCloudProvider.unregister(bridge);
        }
    }

    public BuliCloudBridge getBridge() {
        return bridge;
    }

    public FallbackManager getFallbackManager() {
        return fallbackManager;
    }
}
