package dev.bulicloud.bridge.paper.lifecycle;

import com.fasterxml.jackson.databind.ObjectMapper;
import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.api.BuliCloudProvider;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.client.impl.BuliCloudBridgeImpl;
import dev.bulicloud.bridge.messaging.CloudPacket;
import dev.bulicloud.bridge.messaging.MessageChannel;
import dev.bulicloud.bridge.paper.messaging.CloudNotificationService;

import java.util.concurrent.TimeUnit;
import java.util.logging.Level;
import java.util.logging.Logger;

public class PaperBridgeRuntime {

    private final MessageChannel channel;
    private final BuliCloudBridge bridge;
    private final CloudNotificationService notifications;
    private final Logger logger;

    public PaperBridgeRuntime(Logger logger) {
        this.logger = logger;

        CloudClient client = new CloudClient("http://127.0.0.1:8080/", System.getenv("BRIDGE_TOKEN"));
        channel = client.messageChannel();
        bridge = new BuliCloudBridgeImpl(client);
        notifications = new CloudNotificationService(channel, new ObjectMapper(), System.getenv("INSTANCE_ID"), logger);
    }

    public void start() {
        BuliCloudProvider.register(bridge);
        channel.connect()
                .thenCompose(ignored -> notifications.sendProxyNotification(CloudPacket.SERVER_READY))
                .exceptionally(error -> {
                    logger.log(Level.WARNING, "Paper Bridge initialization failed", error);
                    return null;
                });
        logger.info("[BuliCloud Paper Bridge] paper bridge initialized");
    }

    public void stop() {
        try {
            notifications.sendProxyNotification(CloudPacket.SERVER_STOPPING)
                    .handle((result, error) -> null)
                    .thenCompose(ignored -> channel.disconnect())
                    .get(6, TimeUnit.SECONDS);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        } catch (Exception error) {
            logger.log(Level.WARNING,"Bridge shutdown failed", error);
        } finally {
            BuliCloudProvider.unregister(bridge);
        }
    }

}
