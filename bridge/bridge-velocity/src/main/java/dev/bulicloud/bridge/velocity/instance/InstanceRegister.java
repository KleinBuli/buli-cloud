package dev.bulicloud.bridge.velocity.instance;

import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.server.ServerInfo;
import com.velocitypowered.api.proxy.server.RegisteredServer;
import java.net.InetSocketAddress;
import java.util.HashMap;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Comparator;
import java.util.List;

public class InstanceRegister {
    private final ProxyServer proxy;
    private final Map<String, ServerInfo> managed = new HashMap<>();
    private final Map<String, String> groups = new HashMap<>();

    private List<String> fallbackGroups = List.of();

    public InstanceRegister(ProxyServer proxy) {
        this.proxy = Objects.requireNonNull(proxy);
    }

    public void registerServer(String name, String host, int port) {
        registerServer(name, host, port, name);
    }

    public synchronized void registerServer(String name, String host, int port, String group) {
        if (name == null || name.isBlank() || name.length() > 128 || host == null || host.isBlank() || port < 1 || port > 65535) {
            throw new IllegalArgumentException("Invalid server registration");
        }
        ServerInfo info = new ServerInfo(name, new InetSocketAddress(host, port));
        var existing = proxy.getServer(name);
        if (existing.isPresent() && existing.get().getServerInfo().getAddress().equals(info.getAddress())) {
            managed.put(name, existing.get().getServerInfo());
            groups.put(name, group);
            return;
        }
        existing.ifPresent(server -> proxy.unregisterServer(server.getServerInfo()));
        proxy.registerServer(info);
        managed.put(name, info);
        groups.put(name, group);
    }

    public synchronized Optional<RegisteredServer> findServer(String nameOrGroup) {
        return managed.values().stream()
                .filter(info -> info.getName().equalsIgnoreCase(nameOrGroup)
                        || nameOrGroup.equalsIgnoreCase(groups.get(info.getName())))
                .map(info -> proxy.getServer(info.getName()).filter(server -> server.getServerInfo().equals(info)))
                .flatMap(Optional::stream)
                .min(Comparator.comparingInt((RegisteredServer server) -> server.getPlayersConnected().size())
                        .thenComparing(server -> server.getServerInfo().getName()));
    }

    public synchronized void setFallbackGroups(List<String> groups) {
        fallbackGroups = List.copyOf(groups);
    }

    public synchronized Optional<RegisteredServer> findFallbackServer() {
        if (!fallbackGroups.isEmpty()) {
            return fallbackGroups.stream().map(this::findServer).flatMap(Optional::stream).findFirst();
        }
        return managed.values().stream()
                .map(info -> proxy.getServer(info.getName()).filter(server -> server.getServerInfo().equals(info)))
                .flatMap(Optional::stream)
                .min(Comparator.comparingInt((RegisteredServer server) -> server.getPlayersConnected().size())
                        .thenComparing(server -> server.getServerInfo().getName()));
    }

    public synchronized void unregisterServer(String name) {
        groups.remove(name);
        ServerInfo info = managed.remove(name);
        if (info != null) {
            proxy.getServer(name).filter(server -> server.getServerInfo().equals(info))
                    .ifPresent(server -> proxy.unregisterServer(server.getServerInfo()));
        }
    }
}
