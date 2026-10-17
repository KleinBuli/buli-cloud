package dev.bulicloud.bridge.paper;

import dev.bulicloud.bridge.paper.lifecycle.PaperBridgeRuntime;
import org.bukkit.plugin.java.JavaPlugin;

public class PaperBridge extends JavaPlugin {

    private PaperBridgeRuntime runtime;

    @Override
    public void onEnable() {
        this.runtime = new PaperBridgeRuntime(this.getLogger());
        runtime.start();
        super.onEnable();
    }

    @Override
    public void onDisable() {
        runtime.stop();
        super.onDisable();
    }

    public PaperBridgeRuntime getRuntime() {
        return runtime;
    }
}
