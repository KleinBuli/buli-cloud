package dev.bulicloud.bridge.velocity;

import com.google.inject.Inject;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.event.proxy.ProxyShutdownEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.velocity.lifecycle.VelocityBridgeRuntime;
import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.minimessage.MiniMessage;
import org.slf4j.Logger;

@Plugin(
        id = "bulicloud",
        name = "BuliCloud",
        version = "0.1.0",
        authors = {"KleinBuli"}
)
public final class VelocityBridge {

    private static Component PREFIX = Component.empty();
    private static VelocityBridge instance;

    private final ProxyServer server;
    private final Logger logger;
    private VelocityBridgeRuntime runtime;

    @Inject
    public VelocityBridge(ProxyServer server, Logger logger) {
        instance = this;
        this.server = server;
        this.logger = logger;

        initPrefix();

    }

    @Subscribe
    public void onInitialize(ProxyInitializeEvent event) {
        runtime = new VelocityBridgeRuntime(this, server, logger);
        runtime.start();
    }

    @Subscribe
    public void onStop(ProxyShutdownEvent event) {
        if (runtime != null) {
            runtime.stop();
        }
    }

    public static void initPrefix() {
        PREFIX = MiniMessage.miniMessage().deserialize("<dark_gray>[<color:#8DE3DD>BuliCloud<dark_gray>] ");
    }

    public static Component prefix() {
        return PREFIX;
    }

    public static VelocityBridge getInstance() {
        return instance;
    }

    public BuliCloudBridge getBridge() {
        return getRuntime().getBridge();
    }

    public VelocityBridgeRuntime getRuntime() {
        return runtime;
    }
}
