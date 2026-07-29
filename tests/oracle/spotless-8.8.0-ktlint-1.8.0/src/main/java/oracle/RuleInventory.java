package oracle;

import java.io.File;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Comparator;
import java.util.List;
import java.util.ServiceLoader;

public final class RuleInventory {
    private RuleInventory() {}

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static void main(String[] args) throws Exception {
        if (args.length != 2) {
            throw new IllegalArgumentException("usage: RuleInventory <classpath> <output.json>");
        }

        List<URL> urls = new ArrayList<>();
        for (String entry : args[0].split(File.pathSeparator)) {
            urls.add(Path.of(entry).toUri().toURL());
        }

        List<ProviderInventory> inventories = new ArrayList<>();
        try (URLClassLoader loader = new URLClassLoader(urls.toArray(URL[]::new), ClassLoader.getPlatformClassLoader())) {
            Class<?> providerType = loader.loadClass("com.pinterest.ktlint.cli.ruleset.core.api.RuleSetProviderV3");
            ServiceLoader<?> providers = ServiceLoader.load((Class) providerType, loader);
            for (Object provider : providers) {
                Object providerIdValue = provider.getClass().getMethod("getId").invoke(provider);
                String providerId = providerIdValue.getClass().getMethod("getValue").invoke(providerIdValue).toString();
                Collection<?> ruleProviders = (Collection<?>) provider.getClass().getMethod("getRuleProviders").invoke(provider);
                List<String> ruleIds = new ArrayList<>();
                for (Object ruleProvider : ruleProviders) {
                    Object ruleIdValue = ruleProvider.getClass().getMethod("getRuleId").invoke(ruleProvider);
                    ruleIds.add(ruleIdValue.getClass().getMethod("getValue").invoke(ruleIdValue).toString());
                }
                ruleIds.sort(String::compareTo);
                inventories.add(new ProviderInventory(providerId, ruleIds));
            }
        }
        inventories.sort(Comparator.comparing(ProviderInventory::id));

        Path output = Path.of(args[1]);
        Files.createDirectories(output.getParent());
        Files.writeString(output, toJson(inventories), StandardCharsets.UTF_8);
    }

    private static String toJson(List<ProviderInventory> providers) {
        StringBuilder json = new StringBuilder("{\n  \"schemaVersion\": 1,\n  \"ruleSetProviders\": [\n");
        for (int i = 0; i < providers.size(); i++) {
            ProviderInventory provider = providers.get(i);
            json.append("    {\n      \"id\": \"").append(escape(provider.id())).append("\",\n      \"rules\": [\n");
            for (int j = 0; j < provider.rules().size(); j++) {
                json.append("        \"").append(escape(provider.rules().get(j))).append("\"");
                json.append(j + 1 == provider.rules().size() ? "\n" : ",\n");
            }
            json.append("      ]\n    }");
            json.append(i + 1 == providers.size() ? "\n" : ",\n");
        }
        return json.append("  ]\n}\n").toString();
    }

    private static String escape(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }

    private record ProviderInventory(String id, List<String> rules) {}
}
