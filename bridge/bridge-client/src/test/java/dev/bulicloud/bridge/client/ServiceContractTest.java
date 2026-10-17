package dev.bulicloud.bridge.client;
import com.sun.net.httpserver.HttpServer;
import dev.bulicloud.bridge.client.impl.BuliCloudBridgeImpl;
import dev.bulicloud.bridge.model.enums.GroupRole;
import org.junit.jupiter.api.Test;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeUnit;
import static org.junit.jupiter.api.Assertions.*;

class ServiceContractTest {
    @Test void servicesMatchRustRoutesAndPayloads() throws Exception {
        var server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            String key = exchange.getRequestMethod() + " " + exchange.getRequestURI();
            String body = switch (key) {
                case "GET /health" -> "BuliCloud is running!";
                case "GET /paths" -> "templates: /data/templates";
                case "GET /instances" -> "[{\"instance\":{\"id\":\"lobby-1\",\"group_name\":\"lobby\",\"template_name\":\"default\",\"instance_mode\":\"Dynamic\"},\"status\":\"Running\"}]";
                case "GET /instances/lobby-1" -> "{\"instance\":{\"id\":\"lobby-1\",\"group_name\":\"lobby\",\"template_name\":\"default\",\"instance_mode\":\"Dynamic\"}}";
                case "GET /groups/lobby" -> "{\"name\":\"lobby\",\"role\":\"ProxiedServer\",\"template_names\":[\"default\"],\"maintenance\":false,\"path\":\"/data/lobby\"}";
                case "GET /templates" -> "[\"lobby/default\"]";
                case "GET /templates/lobby/default" -> "{\"name\":\"default\",\"server_software\":\"Paper\",\"min_memory_mb\":512,\"max_memory_mb\":1024,\"role\":null,\"host\":\"127.0.0.1\"}";
                case "POST /groups/lobby?role=Proxy", "POST /templates/lobby/default", "DELETE /templates/lobby/default", "DELETE /instances/lobby-1/remove" -> "";
                default -> null;
            };
            if (body == null) { exchange.sendResponseHeaders(404, -1); }
            else if (body.isEmpty()) { exchange.sendResponseHeaders(204, -1); }
            else { byte[] data=body.getBytes(StandardCharsets.UTF_8); exchange.sendResponseHeaders(200,data.length); exchange.getResponseBody().write(data); }
            exchange.close();
        });
        server.start();
        try {
            var bridge = new BuliCloudBridgeImpl(new CloudClient("http://127.0.0.1:" + server.getAddress().getPort() + "/", "test"));
            assertEquals("BuliCloud is running!", bridge.health().get(3,TimeUnit.SECONDS));
            assertTrue(bridge.paths().get(3,TimeUnit.SECONDS).contains("templates:"));
            assertEquals("lobby",bridge.instances().all().get(3,TimeUnit.SECONDS).getFirst().groupName());
            assertEquals("lobby-1",bridge.instances().get("lobby-1").get(3,TimeUnit.SECONDS).id());
            assertEquals(GroupRole.PROXIED_SERVER,bridge.groups().get("lobby").get(3,TimeUnit.SECONDS).groupRole());
            bridge.groups().create("lobby",GroupRole.PROXY).get(3,TimeUnit.SECONDS);
            bridge.templates().create("lobby","default").get(3,TimeUnit.SECONDS);
            bridge.templates().delete("lobby","default").get(3,TimeUnit.SECONDS);
            assertEquals(512,bridge.templates().all().get(3,TimeUnit.SECONDS).getFirst().minMemoryMb());
            bridge.instances().delete("lobby-1").get(3,TimeUnit.SECONDS);
        } finally { server.stop(0); }
    }
}
