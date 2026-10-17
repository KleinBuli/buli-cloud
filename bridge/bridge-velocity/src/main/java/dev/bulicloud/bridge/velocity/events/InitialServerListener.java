package dev.bulicloud.bridge.velocity.events;

import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.player.PlayerChooseInitialServerEvent;
import com.velocitypowered.api.proxy.ProxyServer;
import dev.bulicloud.bridge.velocity.instance.InstanceRegister;

import java.util.Locale;

public final class InitialServerListener {

    private final ProxyServer server;
    private final InstanceRegister instances;

    public InitialServerListener(ProxyServer server, InstanceRegister instances) {
        this.server = server;
        this.instances = instances;
    }

    @Subscribe
    public void onChooseInitialServer(PlayerChooseInitialServerEvent event) {
        var config = server.getConfiguration();
        var targets = event.getPlayer().getVirtualHost()
                .map(host -> config.getForcedHosts().get(host.getHostString().toLowerCase(Locale.ROOT)))
                .orElse(config.getAttemptConnectionOrder());
        for (String target : targets) {
            var destination = instances.findServer(target);
            if (destination.isPresent()) {
                event.setInitialServer(destination.get());
                return;
            }
        }
    }
}
