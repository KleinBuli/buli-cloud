package dev.bulicloud.bridge.api;

import java.util.Objects;

public final class BuliCloudProvider {

    private BuliCloudProvider() {}

    private static volatile BuliCloudBridge instance;

    public static BuliCloudBridge get() {
        BuliCloudBridge bridge = instance;
        if(bridge == null)
            throw new IllegalStateException("BuliCloud Bridge is not initialized yet.");

        return bridge;
    }

    public static synchronized void register(BuliCloudBridge bridge) {
        if (instance != null) {
            throw new IllegalStateException("BuliCloud Bridge is already registered.");
        }

        instance = Objects.requireNonNull(bridge);
    }
    public static synchronized void unregister(BuliCloudBridge bridge) {
        if(instance == null)
            throw new IllegalStateException("BuliCloud Bridge is not initialized yet.");

        if(instance != bridge)
            throw new IllegalArgumentException("Instance mismatch");

        instance = null;
    }

}
