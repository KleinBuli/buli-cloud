package dev.bulicloud.bridge.velocity.fallback;

import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.server.RegisteredServer;

import java.util.Comparator;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

public class FallbackManager {

    private final ProxyServer proxyServer;
    private final Set<String> fallbackServers = ConcurrentHashMap.newKeySet();

    public FallbackManager(ProxyServer proxyServer) {
        this.proxyServer = proxyServer;
    }

    public void addFallback(String name) {
        fallbackServers.add(name);
    }

    public void removeFallback(String name) {
        fallbackServers.remove(name);
    }

    public Optional<RegisteredServer> selectFallback(String excludedServer) {
        return fallbackServers.stream()
                .filter(name -> !name.equalsIgnoreCase(excludedServer))
                .map(proxyServer::getServer)
                .flatMap(Optional::stream)
                .min(Comparator.comparingInt(server -> server.getPlayersConnected().size()));
    }

}
