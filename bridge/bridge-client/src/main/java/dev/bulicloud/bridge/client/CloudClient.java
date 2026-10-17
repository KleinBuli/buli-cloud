package dev.bulicloud.bridge.client;

import dev.bulicloud.bridge.client.exception.CloudHttpException;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.bulicloud.bridge.client.messaging.WebSocketMessageChannel;
import dev.bulicloud.bridge.messaging.MessageChannel;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;

public final class CloudClient {

    private final HttpClient httpClient;
    private final URI baseUri;
    private final ObjectMapper objectMapper;

    private final MessageChannel messageChannel;
    private final String token;

    public CloudClient(String httpBaseUrl, String token) {

        if (!httpBaseUrl.startsWith("http://") && !httpBaseUrl.startsWith("https://"))
            throw new IllegalArgumentException("HTTP BaseURL should start with http(s)://");

        if(!httpBaseUrl.endsWith("/")) {
            throw new IllegalArgumentException("HTTP BaseURL should stop with / (e. g. http://127.0.0.1:8080/)");
        }

        this.token = java.util.Objects.requireNonNull(token);
        baseUri = URI.create(httpBaseUrl);
        httpClient = HttpClient.newBuilder().build();
        objectMapper = new ObjectMapper();
        messageChannel = new WebSocketMessageChannel(URI.create(("https".equals(baseUri.getScheme()) ? "wss" : "ws") + ":" + baseUri.resolve("bridge").getRawSchemeSpecificPart()), token);
    }

    public <T> CompletableFuture<T> get(String path, TypeReference<T> responseType) {
        HttpRequest request = HttpRequest.newBuilder().timeout(Duration.ofSeconds(10)).uri(this.baseUri.resolve(path)).GET().build();
        return send(request, responseType);
    }

    public <T> CompletableFuture<T> post(String path, Object body, TypeReference<T> responseType) {
        String json;

        try {
            json = objectMapper.writeValueAsString(body);
        } catch (JsonProcessingException exception) {
            return CompletableFuture.failedFuture(exception);
        }

        HttpRequest request = HttpRequest.newBuilder()
                .uri(baseUri.resolve(path))
                .timeout(Duration.ofSeconds(10))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();

        return send(request, responseType);
    }

    public CompletableFuture<Void> delete(String path) {
        HttpRequest request = HttpRequest.newBuilder().timeout(Duration.ofSeconds(10)).uri(this.baseUri.resolve(path)).DELETE().build();
        return send(request, new TypeReference<>() {});
    }

    private <T> CompletableFuture<T> send(HttpRequest request, TypeReference<T> responseType) {
        HttpRequest authenticated = HttpRequest.newBuilder(request, (name, value) -> !name.equalsIgnoreCase("Authorization"))
                .header("Authorization", "Bearer " + token).build();
        return httpClient.sendAsync(authenticated, HttpResponse.BodyHandlers.ofString()).thenApply(response -> {
            if(response.statusCode() < 200 || response.statusCode() >= 300)
                throw new CloudHttpException(response.statusCode(), "HTTP " + response.statusCode() + ": " + response.body());


            if (responseType == null || responseType.getType() == Void.class)
                return null;

            try {
                return objectMapper.readValue(response.body(), responseType);
            } catch (JsonProcessingException exception) {
                throw new CompletionException(exception);
            }
        });
    }

    public MessageChannel messageChannel() {
        return messageChannel;
    }

}

