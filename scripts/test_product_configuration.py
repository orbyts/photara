"""Offline release admission tests; no provider, database, or user storage access."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import generate_product_configuration as product


class ReleaseAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.descriptor = json.loads((product.ROOT / "config/product-identity.json").read_text())

    def remote(self, channel="remoteAcceptance"):
        environment = copy.deepcopy(self.descriptor["environments"]["development"])
        environment.update(channel=channel, environmentId="synthetic-remote",
                           apiOrigin="https://api.example.com/")
        self.descriptor["environments"][channel] = environment
        return environment

    def test_development_and_unprovisioned_release(self):
        result = product.configuration(self.descriptor, "development")
        self.assertEqual(result["environment"]["apiOrigin"], "http://127.0.0.1:8080/")
        for channel in ("production", "remoteAcceptance", "typo"):
            with self.assertRaises(ValueError):
                product.configuration(self.descriptor, channel)

    def test_remote_https_and_checked_development_auth(self):
        environment = self.remote()
        product.configuration(self.descriptor, "remoteAcceptance")
        for bad in ("http://127.0.0.1:8080/", "https://127.0.0.1/", "https://[::1]/",
                    "https://localhost/", "https://localhost./", "https://api.local./",
                    "https://api.local/", "https://api.example.com/path",
                    "https://api.example.com/?x=1", "https://user:password@api.example.com/"):
            environment["apiOrigin"] = bad
            with self.assertRaises(ValueError, msg=bad):
                product.configuration(self.descriptor, "remoteAcceptance")

    def test_production_requires_independent_trust_and_final_brand(self):
        environment = self.remote("production")
        with self.assertRaises(ValueError):
            product.configuration(self.descriptor, "production")
        self.descriptor["identity"]["isCodename"] = False
        with self.assertRaises(ValueError):
            product.configuration(self.descriptor, "production")
        environment.update(auth0Issuer="https://production.example.com/", auth0Audience="urn:synthetic:api",
                           nativeClientId="synthetic-production-client",
                           callbackURL="com.photara.desktop://production.example.com/macos/com.photara.desktop/callback",
                           logoutURL="com.photara.desktop://production.example.com/macos/com.photara.desktop/callback")
        with self.assertRaises(ValueError):
            product.configuration(self.descriptor, "production")
        self.descriptor["identity"].update(callbackScheme="com.example.release", bundleIdentifier="com.example.release")
        environment["callbackURL"] = environment["logoutURL"] = "com.example.release://production.example.com/macos/com.example.release/callback"
        product.configuration(self.descriptor, "production")

    def test_unknown_secret_fields_and_duplicate_keys_refused(self):
        self.descriptor["identity"]["databasePassword"] = "disposable-not-a-real-secret"
        with self.assertRaises(ValueError):
            product.configuration(self.descriptor, "development")
        with self.assertRaises(ValueError):
            json.loads('{"channel":"production","channel":"development"}', object_pairs_hook=product.unique_object)

    def test_coordinate_and_compatibility_refusals(self):
        original = self.descriptor["environments"]["development"]
        for key, bad in (("callbackURL", "wrong://callback"), ("logoutURL", "wrong://logout"),
                         ("minimumAPI", 2), ("telemetryEnabled", True),
                         ("apiOrigin", "http://0.0.0.0:8080/")):
            descriptor = copy.deepcopy(self.descriptor)
            descriptor["environments"]["development"][key] = bad
            with self.assertRaises(ValueError, msg=key):
                product.configuration(descriptor, "development")
        self.assertEqual(original["minimumAPI"], 3)

    def test_brand_cutover_changes_bounded_public_identity(self):
        identity = self.descriptor["identity"]
        identity.update(displayName="Example", shortName="Example", bundleIdentifier="com.example.desktop",
                        callbackScheme="com.example.desktop", projectPackageExtension="exampleproject",
                        applicationSupportDirectory="Example", cacheDirectory="Example")
        env = self.descriptor["environments"]["development"]
        env["callbackURL"] = env["logoutURL"] = "com.example.desktop://dev-nmturasdrkz7up27.us.auth0.com/macos/com.example.desktop/callback"
        result = product.configuration(self.descriptor, "development")
        self.assertEqual(result["identity"]["projectPackageExtension"], "exampleproject")

    def test_brand_path_and_extension_policy_refuses_unsafe_values(self):
        for key in ("productName", "executableName", "applicationSupportDirectory",
                    "cacheDirectory", "defaultProjectsDirectory", "journalDirectory"):
            for bad in ("", "..", "a/b", "a\\b", "a\n", " a", "a:"):
                d = copy.deepcopy(self.descriptor)
                d["identity"][key] = bad
                with self.assertRaises(ValueError, msg=(key, bad)):
                    product.configuration(d, "development")
        for bad in ("", ".jprtest", "../bad", "a.b", "UPPER", "a" * 33):
            d = copy.deepcopy(self.descriptor)
            d["identity"]["projectPackageExtension"] = bad
            with self.assertRaises(ValueError):
                product.configuration(d, "development")
        d = copy.deepcopy(self.descriptor)
        d["identity"]["legacyProjectPackageExtensions"] = ["photara", "photara"]
        with self.assertRaises(ValueError):
            product.configuration(d, "development")

    def test_synthetic_descriptor_keeps_internal_namespace(self):
        from verify_brand_readiness import synthetic_descriptor
        d = synthetic_descriptor()
        selected = product.configuration(d, "development")
        self.assertEqual(selected["environment"]["schemaFamily"], "photara.service.g2")
        self.assertEqual(selected["identity"]["legacyProjectPackageExtensions"], ["photara"])
        for key, value in selected["identity"].items():
            if key != "legacyProjectPackageExtensions":
                self.assertNotIn("photara", str(value).lower(), key)
        self.assertEqual(selected["identity"]["keychainService"], "org.example.juniper.auth")

    def test_shared_generated_source_is_not_rewritten_when_unchanged(self):
        with tempfile.TemporaryDirectory(prefix="product-config-test-") as directory:
            path = Path(directory) / "CheckedReleaseConfiguration.swift"
            product.write_generated(path, "synthetic-public-source")
            original = path.stat().st_mtime_ns
            product.write_generated(path, "synthetic-public-source")
            self.assertEqual(path.stat().st_mtime_ns, original)
            product.write_generated(path, "changed-public-source")
            self.assertEqual(path.read_text(), "changed-public-source")


if __name__ == "__main__":
    unittest.main()
