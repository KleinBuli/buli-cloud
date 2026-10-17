
package dev.bulicloud.bridge.velocity.command.ingame;

import com.velocitypowered.api.command.CommandSource;
import com.velocitypowered.api.command.SimpleCommand;
import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.client.exception.CloudHttpException;
import dev.bulicloud.bridge.model.enums.GroupRole;
import dev.bulicloud.bridge.velocity.VelocityBridge;
import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.format.NamedTextColor;
import org.slf4j.Logger;

import java.util.*;
import java.util.concurrent.*;
import java.util.function.Function;

public final class CloudCommand implements SimpleCommand {

    private static final String BASE_PERMISSION = "bulicloud.command.cloud";
    private static final String COMMAND_PREFIX = "/bulicloud ";

    private final Logger logger;
    private final List<Operation> operations = new ArrayList<>();

    private record Operation(
            String key,
            String usage,
            int minArgs,
            int maxArgs,
            Function<String[], CompletableFuture<List<String>>> action) {
        String permission() {
            return BASE_PERMISSION + "." + key.replace(' ', '.');
        }
    }

    public CloudCommand(BuliCloudBridge bridge, Logger logger) {
        Objects.requireNonNull(bridge, "Bridge cannot be null");
        this.logger = Objects.requireNonNull(logger, "Logger cannot be null");

        registerInstanceCommands(bridge);
        registerGroupCommands(bridge);
        registerTemplateCommands(bridge);
        registerPlayerCommands(bridge);
        registerSystemCommands(bridge);
    }

    // command registration

    private void registerInstanceCommands(BuliCloudBridge bridge) {

        add("instance list", "", 0, 0, args ->
                bridge.instances().all().thenApply(instances ->
                        instances.stream()
                                .map(instance -> String.format(
                                        "%s | %s/%s | %s",
                                        instance.id(),
                                        instance.groupName(),
                                        instance.template_name(),
                                        instance.instanceMode()
                                ))
                                .toList()
                )
        );

        add("instance info", "<id>", 1, 1, args ->
                bridge.instances().get(name(args[0])).thenApply(instance ->
                        List.of(String.format(
                                "%s | %s/%s | %s",
                                instance.id(),
                                instance.groupName(),
                                instance.template_name(),
                                instance.instanceMode()
                        ))
                )
        );

        add("instance create", "<group> <template>", 2, 2, args ->
                done(bridge.instances().create(name(args[0]), name(args[1])))
        );

        add("instance start", "<id>", 1, 1, args ->
                done(bridge.instances().start(name(args[0])))
        );

        add("instance stop", "<id>", 1, 1, args ->
                done(bridge.instances().stop(name(args[0])))
        );

        add("instance delete", "<id>", 1, 1, args ->
                done(bridge.instances().delete(name(args[0])))
        );
    }

    private void registerGroupCommands(BuliCloudBridge bridge) {

        add("group list", "", 0, 0, args ->
                bridge.groups().all().thenApply(groups ->
                        groups.stream()
                                .map(group -> String.format(
                                        "%s | %s | Maintenance: %s",
                                        group.groupName(),
                                        group.groupRole(),
                                        group.maintenance()
                                ))
                                .toList()
                )
        );

        add("group info", "<name>", 1, 1, args ->
                bridge.groups().get(name(args[0])).thenApply(group ->
                        List.of(
                                String.format(
                                        "%s | %s | Maintenance: %s",
                                        group.groupName(),
                                        group.groupRole(),
                                        group.maintenance()
                                ),
                                "Templates: " + group.template_names()
                        )
                )
        );

        add("group create", "<name> <Proxy|ProxiedServer|StandaloneServer>", 2, 2, args ->
                done(bridge.groups().create(name(args[0]), role(args[1])))
        );

        add("group delete", "<name>", 1, 1, args ->
                done(bridge.groups().delete(name(args[0])))
        );

        add("group maintenance", "<name> <on|off>", 2, 2, args -> {
            String state = args[1];

            if (!state.equalsIgnoreCase("on") && !state.equalsIgnoreCase("off")) {
                throw new IllegalArgumentException("Maintenance must be 'on' or 'off'.");
            }

            return done(bridge.groups().setMaintenance(
                    name(args[0]),
                    state.equalsIgnoreCase("on")
            ));
        });
    }

    private void registerTemplateCommands(BuliCloudBridge bridge) {

        add("template list", "[group]", 0, 1, args -> {
            if (args.length == 1) {
                return bridge.groups().get(name(args[0])).thenApply(group ->
                        group.template_names().stream()
                                .map(template -> group.groupName() + "/" + template)
                                .toList()
                );
            }

            return bridge.groups().all().thenApply(groups ->
                    groups.stream()
                            .flatMap(group -> group.template_names().stream()
                                    .map(template -> group.groupName() + "/" + template))
                            .toList()
            );
        });

        add("template info", "<group> <name>", 2, 2, args ->
                bridge.templates().get(name(args[0]), name(args[1])).thenApply(template ->
                        List.of(
                                String.format(
                                        "%s | %s | Minecraft: %s",
                                        template.name(),
                                        template.serverSoftware(),
                                        template.minecraftVersion()
                                ),
                                String.format(
                                        "RAM: %d-%d MB | Instances: %d-%d",
                                        template.minMemoryMb(),
                                        template.maxMemoryMb(),
                                        template.minInstances(),
                                        template.maxInstances()
                                )
                        )
                )
        );

        add("template create", "<group> <name>", 2, 2, args ->
                done(bridge.templates().create(name(args[0]), name(args[1])))
        );

        add("template delete", "<group> <name>", 2, 2, args ->
                done(bridge.templates().delete(name(args[0]), name(args[1])))
        );
    }

    private void registerPlayerCommands(BuliCloudBridge bridge) {

        add("player list", "", 0, 0, args ->
                bridge.cloudPlayers().all().thenApply(players ->
                        players.stream()
                                .map(player -> String.format(
                                        "%s | %s | %s",
                                        player.name(),
                                        player.uuid(),
                                        player.instance_name()
                                ))
                                .toList()
                )
        );

        add("player info", "<uuid>", 1, 1, args ->
                bridge.cloudPlayers().get(uuid(args[0])).thenApply(player ->
                        List.of(String.format(
                                "%s | %s | Proxy: %s | Instance: %s",
                                player.name(),
                                player.uuid(),
                                player.proxy_name(),
                                player.instance_name()
                        ))
                )
        );

        add("player transfer", "<uuid> <instance>", 2, 2, args ->
                done(bridge.cloudPlayers().transfer(
                        uuid(args[0]),
                        name(args[1])
                ))
        );

        add("player disconnect", "<uuid> <reason...>", 2, Integer.MAX_VALUE, args -> {
            String reason = String.join(" ", Arrays.copyOfRange(args, 1, args.length));

            return done(bridge.cloudPlayers().disconnect(
                    uuid(args[0]),
                    reason
            ));
        });
    }

    private void registerSystemCommands(BuliCloudBridge bridge) {

        add("health", "", 0, 0, args ->
                bridge.health().thenApply(List::of)
        );

        add("paths", "", 0, 0, args ->
                bridge.paths().thenApply(result -> result.lines().toList())
        );

        add("reload", "", 0, 0, args ->
                done(bridge.reload())
        );

        add("shutdown", "", 0, 0, args ->
                done(bridge.shutdown())
        );
    }

    private void add(
            String key,
            String usage,
            int minArgs,
            int maxArgs,
            Function<String[], CompletableFuture<List<String>>> action
    ) {
        operations.add(new Operation(key, usage, minArgs, maxArgs, action));
    }

   // Execution

    @Override
    public void execute(Invocation invocation) {
        CommandSource source = invocation.source();
        String[] args = invocation.arguments();

        if (!hasPermission(invocation)) {
            send(source, "You do not have permission to use this command.", NamedTextColor.RED);
            return;
        }

        if (args.length == 0 || (args.length == 1 && args[0].equalsIgnoreCase("help"))) {
            sendHelp(source);
            return;
        }

        for (Operation operation : operations) {
            String[] words = operation.key().split(" ");

            if (!matches(args, words)) {
                continue;
            }

            if (!source.hasPermission(operation.permission())) {
                send(source, "Missing permission: " + operation.permission(), NamedTextColor.RED);
                return;
            }

            String[] values = Arrays.copyOfRange(args, words.length, args.length);

            if (values.length < operation.minArgs() || values.length > operation.maxArgs()) {
                sendUsage(source, operation);
                return;
            }

            executeOperation(source, operation, values);
            return;
        }

        send(source, "Unknown command. Use /bulicloud help.", NamedTextColor.RED);
    }

    private void executeOperation(CommandSource source, Operation operation, String[] args) {
        CompletableFuture<List<String>> request;

        try {
            request = operation.action().apply(args);
        } catch (IllegalArgumentException exception) {
            send(source, exception.getMessage(), NamedTextColor.RED);
            sendUsage(source, operation);
            return;
        } catch (RuntimeException exception) {
            handleFailure(source, operation, exception);
            return;
        }

        send(source, "Processing request...", NamedTextColor.GRAY);

        request.whenComplete((lines, error) -> {
            if (error != null) {
                handleFailure(source, operation, error);
                return;
            }

            if (lines == null || lines.isEmpty()) {
                send(source, "No entries found.", NamedTextColor.GRAY);
                return;
            }

            lines.forEach(line -> send(source, line, NamedTextColor.GRAY));
        });
    }

    private static boolean matches(String[] args, String[] words) {
        if (args.length < words.length) {
            return false;
        }

        for (int i = 0; i < words.length; i++) {
            if (!words[i].equalsIgnoreCase(args[i])) {
                return false;
            }
        }

        return true;
    }

   // Perms and Suggestions

    @Override
    public boolean hasPermission(Invocation invocation) {
        return invocation.source().hasPermission(BASE_PERMISSION);
    }

    @Override
    public List<String> suggest(Invocation invocation) {
        if (!hasPermission(invocation)) {
            return List.of();
        }

        String[] args = invocation.arguments();
        String input = String.join(" ", args).toLowerCase(Locale.ROOT);
        int index = Math.max(0, args.length - 1);

        List<String> commands = new ArrayList<>();
        commands.add("help");

        operations.stream()
                .filter(operation -> invocation.source().hasPermission(operation.permission()))
                .map(Operation::key)
                .forEach(commands::add);

        return commands.stream()
                .filter(command -> command.startsWith(input))
                .map(command -> command.split(" "))
                .filter(words -> words.length > index)
                .map(words -> words[index])
                .distinct()
                .sorted()
                .toList();
    }

    // Help messages

    private void sendHelp(CommandSource source) {
        send(source, "BuliCloud - Command Help", NamedTextColor.AQUA);
        send(source, COMMAND_PREFIX + "help", NamedTextColor.GRAY);

        operations.stream()
                .filter(operation -> source.hasPermission(operation.permission()))
                .forEach(operation -> sendUsage(source, operation));
    }

    private static void sendUsage(CommandSource source, Operation operation) {
        String usage = COMMAND_PREFIX + operation.key();

        if (!operation.usage().isEmpty()) {
            usage += " " + operation.usage();
        }

        send(source, usage, NamedTextColor.YELLOW);
    }

    private static void send(CommandSource source, String message, NamedTextColor color) {
        source.sendMessage(
                VelocityBridge.prefix().append(Component.text(message, color))
        );
    }

    // Error Handling

    private void handleFailure(CommandSource source, Operation operation, Throwable error) {
        Throwable cause = unwrap(error);

        logger.warn(
                "BuliCloud command failed: {} ({})",
                operation.key(),
                cause.getClass().getSimpleName(),
                cause
        );

        String message;

        if (cause instanceof CloudHttpException http) {
            message = switch (http.statusCode()) {
                case 400, 422 -> "The cloud rejected the provided arguments.";
                case 401, 403 -> "API access was denied by the cloud.";
                case 404 -> "The requested resource was not found.";
                case 409 -> "Resource already exists or operation is not allowed in the current state.";
                default -> "Cloud request failed (HTTP " + http.statusCode() + ").";
            };
        } else if (cause instanceof java.net.http.HttpTimeoutException
                || cause instanceof TimeoutException) {
            message = "Request timed out. Check the current state before retrying.";
        } else {
            message = "Cloud request failed. Check the connection and cloud logs.";
        }

        send(source, message, NamedTextColor.RED);
    }

    private static Throwable unwrap(Throwable error) {
        while ((error instanceof CompletionException || error instanceof ExecutionException)
                && error.getCause() != null) {
            error = error.getCause();
        }

        return error;
    }

   // Utils

    private static CompletableFuture<List<String>> done(CompletableFuture<Void> request) {
        return request.thenApply(ignored ->
                List.of("Operation successfully confirmed by the cloud.")
        );
    }

    private static String name(String value) {
        String reserved = value.split("\\.", -1)[0].toUpperCase(Locale.ROOT);

        boolean valid = value.matches("[A-Za-z0-9_.-]+")
                && !value.endsWith(".")
                && !reserved.matches("CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9]");

        if (!valid) {
            throw new IllegalArgumentException("Invalid name or ID: " + value);
        }

        return value;
    }

    private static String uuid(String value) {
        try {
            String canonical = UUID.fromString(value).toString();

            if (!canonical.equalsIgnoreCase(value)) {
                throw new IllegalArgumentException();
            }

            return canonical;
        } catch (IllegalArgumentException exception) {
            throw new IllegalArgumentException("Invalid UUID.");
        }
    }

    private static GroupRole role(String value) {
        return switch (value.replace("_", "").toLowerCase(Locale.ROOT)) {
            case "proxy" -> GroupRole.PROXY;
            case "proxiedserver" -> GroupRole.PROXIED_SERVER;
            case "standaloneserver" -> GroupRole.STANDALONE_SERVER;
            default -> throw new IllegalArgumentException(
                    "Role must be Proxy, ProxiedServer or StandaloneServer."
            );
        };
    }
}
