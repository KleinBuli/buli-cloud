package dev.bulicloud.bridge.velocity.fallback;

import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.player.KickedFromServerEvent;
import com.velocitypowered.api.event.player.PlayerChooseInitialServerEvent;
import net.kyori.adventure.text.Component;

public class FallbackListener {

    private final FallbackManager fallbackManager;

    public FallbackListener(FallbackManager fallbackManager) {
        this.fallbackManager = fallbackManager;
    }

    @Subscribe
    public void handleInitialConnect(PlayerChooseInitialServerEvent event) {
        fallbackManager.selectFallback(null).ifPresent(event::setInitialServer);
    }

    @Subscribe
    public void handleKickedFromServer(KickedFromServerEvent event) {
        String excluded = event.getServer().getServerInfo().getName();

        fallbackManager.selectFallback(excluded).ifPresentOrElse(
                server -> event.setResult(KickedFromServerEvent.RedirectPlayer.create(server)),
                () -> event.setResult(KickedFromServerEvent.DisconnectPlayer.create(Component.text("No fallback server available.")))
        );
    }

}
