import unittest

from exonum_client import ExonumClient
from exonum_client.crypto import KeyPair
from exonum_launcher.configuration import Configuration
from exonum_launcher.launcher import Launcher

from suite import (
    assert_processes_exited_successfully,
    run_4_nodes,
    wait_network_to_start,
    ExonumCryptoAdvancedClient,
    generate_config,
)


class FreezeTests(unittest.TestCase):
    """Tests for a checking service freezing mechanism."""

    def setUp(self):
        try:
            self.network = run_4_nodes("exonum-cryptocurrency-advanced")
            wait_network_to_start(self.network)
        except Exception as error:
            # If exception is raise in `setUp`, `tearDown` won't be called,
            # thus here we ensure that network is stopped and temporary data is removed.
            # Then we re-raise exception, since the test should fail.
            self.network.stop()
            self.network.deinitialize()
            raise error

    def test_freeze_service(self):

        host, public_port, private_port = self.network.api_address(0)
        client = ExonumClient(host, public_port, private_port)

        # Create wallet
        alice_keys = KeyPair.generate()
        with ExonumCryptoAdvancedClient(client) as crypto_client:
            crypto_client.create_wallet(alice_keys, "Alice")
            with client.create_subscriber("transactions") as subscriber:
                subscriber.wait_for_new_event()
                alice_balance = crypto_client.get_balance(alice_keys)
                self.assertEqual(alice_balance, 100)

        # Freeze the service
        instances = {"crypto": {"artifact": "cryptocurrency", "action": "freeze"}}
        cryptocurrency_advanced_config_dict = generate_config(self.network, instances=instances, deploy=False)

        cryptocurrency_advanced_config = Configuration(cryptocurrency_advanced_config_dict)
        with Launcher(cryptocurrency_advanced_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()
            launcher.start_all()
            launcher.wait_for_start()

        # Check that the service status has been changed for `frozen`.
        for service in client.public_api.available_services().json()["services"]:
            if service["spec"]["name"] == "crypto":
                self.assertEqual(service["status"]["type"], "frozen")

        # Try to create a new wallet. The operation should be failed.
        with ExonumCryptoAdvancedClient(client) as crypto_client:
            bob_keys = KeyPair.generate()
            response = crypto_client.create_wallet(bob_keys, "Bob")
            self.assertEqual(response.status_code, 400)
            self.assertEqual(
                response.json()["title"], "Failed to add transaction to memory pool"  # Cause the service is frozen
            )

        # Check that we can use service endpoints for data retrieving. Check wallet once again.
        with ExonumCryptoAdvancedClient(client) as crypto_client:
            alice_balance = crypto_client.get_balance(alice_keys)
            self.assertEqual(alice_balance, 100)

    def test_resume_after_freeze_service(self):

        host, public_port, private_port = self.network.api_address(0)
        client = ExonumClient(host, public_port, private_port)

        # Create wallet
        with ExonumCryptoAdvancedClient(client) as crypto_client:
            alice_keys = KeyPair.generate()
            crypto_client.create_wallet(alice_keys, "Alice")
            with client.create_subscriber("transactions") as subscriber:
                subscriber.wait_for_new_event()
                alice_balance = crypto_client.get_balance(alice_keys)
                self.assertEqual(alice_balance, 100)

        # Freeze the service
        instances = {"crypto": {"artifact": "cryptocurrency", "action": "freeze"}}
        cryptocurrency_advanced_config_dict = generate_config(self.network, instances=instances, deploy=False)

        cryptocurrency_advanced_config = Configuration(cryptocurrency_advanced_config_dict)
        with Launcher(cryptocurrency_advanced_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()
            launcher.start_all()
            launcher.wait_for_start()

        # Check that the service status has been changed for `frozen`.
        for service in client.public_api.available_services().json()["services"]:
            if service["spec"]["name"] == "crypto":
                self.assertEqual(service["status"]["type"], "frozen")

        # Try to create a new wallet. The operation should be failed.
        with ExonumCryptoAdvancedClient(client) as crypto_client:
            bob_keys = KeyPair.generate()
            response = crypto_client.create_wallet(bob_keys, "Bob")
            self.assertEqual(response.status_code, 400)
            self.assertEqual(
                response.json()["title"], "Failed to add transaction to memory pool"  # Cause the service is frozen
            )

        # Resume the service
        instances = {"crypto": {"artifact": "cryptocurrency", "action": "resume"}}
        cryptocurrency_advanced_config_dict = generate_config(self.network, instances=instances, deploy=False)

        cryptocurrency_advanced_config = Configuration(cryptocurrency_advanced_config_dict)
        with Launcher(cryptocurrency_advanced_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()
            launcher.start_all()
            launcher.wait_for_start()

        # Check that the service status has been changed for `frozen`.
        for service in client.public_api.available_services().json()["services"]:
            if service["spec"]["name"] == "crypto":
                self.assertEqual(service["status"]["type"], "active")

        # Check that an ability to create wallets has been restored.
        with ExonumCryptoAdvancedClient(client) as crypto_client:
            bob_keys = KeyPair.generate()
            crypto_client.create_wallet(bob_keys, "Bob")
            with client.create_subscriber("transactions") as subscriber:
                subscriber.wait_for_new_event()
                alice_balance = crypto_client.get_balance(bob_keys)
                self.assertEqual(alice_balance, 100)

    def tearDown(self):
        outputs = self.network.stop()
        assert_processes_exited_successfully(self, outputs)
        self.network.deinitialize()
