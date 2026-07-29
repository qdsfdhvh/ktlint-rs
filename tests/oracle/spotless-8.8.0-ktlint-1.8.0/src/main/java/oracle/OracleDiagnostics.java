package oracle;

import com.pinterest.ktlint.rule.engine.api.Code;
import com.pinterest.ktlint.rule.engine.api.EditorConfigDefaults;
import com.pinterest.ktlint.rule.engine.api.EditorConfigOverride;
import com.pinterest.ktlint.rule.engine.api.KtLintRuleEngine;
import com.pinterest.ktlint.rule.engine.api.LintError;
import com.pinterest.ktlint.ruleset.standard.StandardRuleSetProvider;
import java.nio.charset.StandardCharsets;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import kotlin.Unit;

public final class OracleDiagnostics {
    private OracleDiagnostics() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 3) {
            throw new IllegalArgumentException("usage: OracleDiagnostics <root> <output.json> <exit-code.txt>");
        }
        Path root = Path.of(args[0]).toAbsolutePath().normalize();
        Path output = Path.of(args[1]);
        Path exitCode = Path.of(args[2]);

        KtLintRuleEngine engine = new KtLintRuleEngine(
                new StandardRuleSetProvider().getRuleProviders(),
                EditorConfigDefaults.Companion.getEMPTY_EDITOR_CONFIG_DEFAULTS(),
                EditorConfigOverride.Companion.getEMPTY_EDITOR_CONFIG_OVERRIDE(),
                true,
                FileSystems.getDefault());

        List<Diagnostic> diagnostics = new ArrayList<>();
        try (var paths = Files.walk(root.resolve("src"))) {
            for (Path path : paths.filter(Files::isRegularFile).sorted().toList()) {
                Path relative = root.relativize(path);
                boolean generated = false;
                for (Path segment : relative) {
                    generated |= segment.toString().equals("generated");
                }
                if (!path.toString().endsWith(".kt") || generated) {
                    continue;
                }
                engine.lint(Code.Companion.fromFile(path.toFile()), error -> {
                    diagnostics.add(Diagnostic.from(relative, error));
                    return Unit.INSTANCE;
                });
            }
        }
        diagnostics.sort(Comparator.comparing(Diagnostic::file)
                .thenComparingInt(Diagnostic::line)
                .thenComparingInt(Diagnostic::column)
                .thenComparing(Diagnostic::rule)
                .thenComparing(Diagnostic::message));

        Files.createDirectories(output.getParent());
        Files.writeString(output, toJson(diagnostics), StandardCharsets.UTF_8);
        Files.writeString(exitCode, diagnostics.isEmpty() ? "0\n" : "1\n", StandardCharsets.UTF_8);
    }

    private static String toJson(List<Diagnostic> diagnostics) {
        StringBuilder json = new StringBuilder("[\n");
        for (int i = 0; i < diagnostics.size(); i++) {
            Diagnostic diagnostic = diagnostics.get(i);
            json.append("  {\n")
                    .append("    \"auto_fixable\": ").append(diagnostic.autoFixable()).append(",\n")
                    .append("    \"column\": ").append(diagnostic.column()).append(",\n")
                    .append("    \"file\": \"").append(escape(diagnostic.file())).append("\",\n")
                    .append("    \"line\": ").append(diagnostic.line()).append(",\n")
                    .append("    \"message\": \"").append(escape(diagnostic.message())).append("\",\n")
                    .append("    \"rule\": \"").append(escape(diagnostic.rule())).append("\"\n")
                    .append("  }");
            json.append(i + 1 == diagnostics.size() ? "\n" : ",\n");
        }
        return json.append("]\n").toString();
    }

    private static String escape(String value) {
        return value.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }

    private record Diagnostic(
            boolean autoFixable,
            int column,
            String file,
            int line,
            String message,
            String rule) {
        private static Diagnostic from(Path relative, LintError error) {
            return new Diagnostic(
                    error.getCanBeAutoCorrected(),
                    error.getCol(),
                    relative.toString().replace('\\', '/'),
                    error.getLine(),
                    error.getDetail(),
                    error.getRuleId().getValue());
        }
    }
}
