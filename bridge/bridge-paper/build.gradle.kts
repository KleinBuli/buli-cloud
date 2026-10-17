plugins {
    id("java")
}

group = "dev.bulicloud"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
    maven("https://repo.papermc.io/repository/maven-public/")
}

dependencies {
    testImplementation(platform("org.junit:junit-bom:5.10.0"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    implementation(project(":bridge-common"))
    implementation(project(":bridge-client"))
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.20.0")
    compileOnly("io.papermc.paper:paper-api:26.2.build.+")
}

tasks.jar {
    archiveFileName.set("bulicloud-bridge-paper.jar")
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
    manifest.attributes["Multi-Release"] = "true"
    from(configurations.runtimeClasspath.get().map { if (it.isDirectory) it else zipTree(it) })
    exclude("META-INF/*.SF", "META-INF/*.RSA", "META-INF/*.DSA", "module-info.class", "META-INF/versions/**/module-info.class")
}

tasks.test { useJUnitPlatform() }