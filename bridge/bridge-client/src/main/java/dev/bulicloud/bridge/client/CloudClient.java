package dev.bulicloud.bridge.client;

import dev.bulicloud.bridge.client.exception.CloudHttpException;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;

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

    public CloudClient(String httpBaseUrl) {

        if (!httpBaseUrl.startsWith("http://") || !httpBaseUrl.startsWith("https://"))
            throw new IllegalArgumentException("HTTP BaseURL should start with http(s)://");

        baseUri = URI.create(httpBaseUrl);
        httpClient = HttpClient.newBuilder().build();
        objectMapper = new ObjectMapper();
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
        return send(request, new TypeReference<Void>() {});
    }

    private <T> CompletableFuture<T> send(HttpRequest request, TypeReference<T> responseType) {
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString()).thenApply(response -> {
            if(response.statusCode() < 200 || response.statusCode() >= 300)
                throw new CloudHttpException(response.statusCode(), "HTTP " + response.statusCode() + ": " + response.body());


            if (responseType == null)
                return null;

            try {
                return objectMapper.readValue(response.body(), responseType);
            } catch (JsonProcessingException exception) {
                throw new CompletionException(exception);
            }

        });
    }

}

