"""Verify Evita AppImage release update metadata without building an image."""

import unittest

import build


class UpdateMetadataTest(unittest.TestCase):
    def test_stable_release_uses_evita_repository(self):
        name, information, url = build.update_metadata("0.1.1", "v0.1.1")

        self.assertEqual(name, "evita-v0.1.1-Linux-X64.AppImage")
        self.assertEqual(
            information,
            "gh-releases-zsync|tsukumivi|Evita|latest|evita-*-Linux-X64.AppImage.zsync",
        )
        self.assertEqual(
            url,
            "https://github.com/tsukumivi/Evita/releases/download/v0.1.1/evita-v0.1.1-Linux-X64.AppImage",
        )

    def test_prerelease_tracks_prerelease_channel(self):
        _, information, _ = build.update_metadata("0.1.1-nightly.20261107", "v0.1.1-nightly.20261107")

        self.assertIn("|latest-pre|", information)

    def test_mismatched_tag_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "must match"):
            build.update_metadata("0.1.1", "v0.1.2")


if __name__ == "__main__":
    unittest.main()
