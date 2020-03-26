import time
import unittest

from exonum_client import ExonumClient
from exonum_launcher.action_result import ActionResult
from exonum_launcher.configuration import Configuration
from exonum_launcher.explorer import NotCommittedError
from exonum_launcher.launcher import Launcher

from suite import (
    assert_processes_exited_successfully,
    run_4_nodes,
    wait_network_to_start,
    generate_config,
    generate_migration_config,
)

INSTANCE_NAME = "cryptocurrency"


class MigrationTests(unittest.TestCase):
    """Tests for a checking service migration mechanism."""

    def setUp(self):
        self.network = run_4_nodes("cryptocurrency-migration")
        self.addCleanup(self._tear_down, False)
        wait_network_to_start(self.network)

    def wait_for_api_restart(self):
        """Waits until the API servers of nodes are restarted after the set
        of active services has changed."""

        time.sleep(0.25)
        wait_network_to_start(self.network)

    def test_full_migration_flow(self):
        """Tests full service migration flow."""

        host, public_port, private_port = self.network.api_address(0)
        client = ExonumClient(host, public_port, private_port)

        # Deploy a service with 0.2.0 version.
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency"}}
        config_dict = generate_config(self.network, instances=instances)
        deploy_config = Configuration(config_dict)

        with Launcher(deploy_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()

            self.wait_for_api_restart()
            explorer = launcher.explorer()
            for artifact in launcher.launch_state.completed_deployments():
                deployed = explorer.is_deployed(artifact)
                self.assertEqual(deployed, True)

        # Stop the working service with version 0.1.0.
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency", "action": "stop"}}
        stop_config_dict = generate_config(
            self.network, instances=instances, artifact_action="none", artifact_version="0.1.0"
        )
        stop_config = Configuration(stop_config_dict)

        with Launcher(stop_config) as launcher:
            launcher.start_all()
            launcher.wait_for_start()

            self.wait_for_api_restart()
            # Check that the service status has been changed to `stopped`.
            for service in client.public_api.available_services().json()["services"]:
                if service["spec"]["name"] == INSTANCE_NAME:
                    self.assertEqual(service["status"]["type"], "stopped")

        # Migrate service data from 0.1.0 to 0.2.0 version
        migrations = {INSTANCE_NAME: {"runtime": "rust", "name": "exonum-cryptocurrency", "version": "0.2.0"}}
        migrations_dict = generate_migration_config(self.network, migrations)
        migration_config = Configuration(migrations_dict)

        with Launcher(migration_config) as launcher:
            launcher.migrate_all()
            launcher.wait_for_migration()

            for service in client.public_api.available_services().json()["services"]:
                if service["spec"]["name"] == INSTANCE_NAME:
                    self.assertEqual(service["data_version"], "0.2.0")

        # Migrate service logic from 0.1.0 to 0.2.0 version
        with Launcher(migration_config) as launcher:
            launcher.migrate_all()
            launcher.wait_for_migration()

            for service in client.public_api.available_services().json()["services"]:
                if service["spec"]["name"] == INSTANCE_NAME:
                    self.assertEqual(service["spec"]["artifact"]["version"], "0.2.0")

        # Resume service with a new logic version 0.2.0
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency", "action": "resume"}}
        resume_config_dict = generate_config(
            self.network, instances=instances, artifact_action="none"
        )
        resume_config = Configuration(resume_config_dict)

        with Launcher(resume_config) as launcher:
            launcher.start_all()
            launcher.wait_for_start()

            self.wait_for_api_restart()
            # Check that the service status has been changed to `active`.
            for service in client.public_api.available_services().json()["services"]:
                if service["spec"]["name"] == INSTANCE_NAME:
                    self.assertEqual(service["status"]["type"], "active")
                    self.assertEqual(service["spec"]["artifact"]["version"], "0.2.0")

        # Unload artifact with version 0.1.0
        unload_config_dict = generate_config(
            self.network, instances=instances, artifact_action="unload", artifact_version="0.1.0"
        )
        unload_config = Configuration(unload_config_dict)

        with Launcher(unload_config) as launcher:
            launcher.unload_all()
            launcher.wait_for_unload()

            self.wait_for_api_restart()
            explorer = launcher.explorer()

            for artifact in unload_config.artifacts.values():
                deployed = explorer.is_deployed(artifact)
                self.assertEqual(deployed, False)

    def test_migrate_running_service(self):
        """Tests migration flow when the migrating service is running."""

        # Deploy a service with 0.2.0 version.
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency"}}
        config_dict = generate_config(self.network, instances=instances)
        deploy_config = Configuration(config_dict)

        with Launcher(deploy_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()

            self.wait_for_api_restart()
            explorer = launcher.explorer()
            for artifact in launcher.launch_state.completed_deployments():
                deployed = explorer.is_deployed(artifact)
                self.assertEqual(deployed, True)

        # Migrate service data from 0.1.0 to 0.2.0 version
        migrations = {INSTANCE_NAME: {"runtime": "rust", "name": "exonum-cryptocurrency", "version": "0.2.0"}}
        migrations_dict = generate_migration_config(self.network, migrations)
        migration_config = Configuration(migrations_dict)

        with Launcher(migration_config) as launcher:
            launcher.migrate_all()
            launcher.wait_for_migration()

            for instance, (status, message) in launcher.launch_state.completed_migrations().items():
                if instance == INSTANCE_NAME:
                    self.assertEqual(status, ActionResult.Fail)
                    self.assertTrue("is not stopped or frozen" in message)

    def test_migrate_without_migration_logic_stage(self):
        """Tests migration flow without migration logic stage."""

        host, public_port, private_port = self.network.api_address(0)
        client = ExonumClient(host, public_port, private_port)

        # Deploy a service with 0.2.0 version.
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency"}}
        config_dict = generate_config(self.network, instances=instances)
        deploy_config = Configuration(config_dict)

        with Launcher(deploy_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()

            self.wait_for_api_restart()
            explorer = launcher.explorer()
            for artifact in launcher.launch_state.completed_deployments():
                deployed = explorer.is_deployed(artifact)
                self.assertEqual(deployed, True)

        # Stop the working service with version 0.1.0.
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency", "action": "stop"}}
        stop_config_dict = generate_config(
            self.network, instances=instances, artifact_action="none", artifact_version="0.1.0"
        )
        stop_config = Configuration(stop_config_dict)

        with Launcher(stop_config) as launcher:
            launcher.start_all()
            launcher.wait_for_start()

            self.wait_for_api_restart()
            # Check that the service status has been changed to `stopped`.
            for service in client.public_api.available_services().json()["services"]:
                if service["spec"]["name"] == INSTANCE_NAME:
                    self.assertEqual(service["status"]["type"], "stopped")

        # Migrate service data from 0.1.0 to 0.2.0 version
        migrations = {INSTANCE_NAME: {"runtime": "rust", "name": "exonum-cryptocurrency", "version": "0.2.0"}}
        migrations_dict = generate_migration_config(self.network, migrations)
        migration_config = Configuration(migrations_dict)

        with Launcher(migration_config) as launcher:
            launcher.migrate_all()
            launcher.wait_for_migration()

            for service in client.public_api.available_services().json()["services"]:
                if service["spec"]["name"] == INSTANCE_NAME:
                    self.assertEqual(service["data_version"], "0.2.0")

        # Try to resume the service without a new logic migration to version 0.2.0
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency", "action": "resume"}}
        resume_config_dict = generate_config(
            self.network, instances=instances, artifact_action="none"
        )
        resume_config = Configuration(resume_config_dict)

        with Launcher(resume_config) as launcher:
            launcher.start_all()
            with self.assertRaises(NotCommittedError) as e:
                launcher.wait_for_start()
                self.assertTrue(
                    f"Service `{INSTANCE_NAME}` has data version (0.2.0) differing from its artifact version" in e
                )

    def test_unload_artifact_of_running_service(self):
        """Tests unload logic when running service references to an artifact."""

        # Deploy a service with 0.2.0 version.
        instances = {INSTANCE_NAME: {"artifact": "cryptocurrency"}}
        config_dict = generate_config(self.network, instances=instances)
        deploy_config = Configuration(config_dict)

        with Launcher(deploy_config) as launcher:
            launcher.deploy_all()
            launcher.wait_for_deploy()

            self.wait_for_api_restart()
            explorer = launcher.explorer()
            for artifact in launcher.launch_state.completed_deployments():
                deployed = explorer.is_deployed(artifact)
                self.assertEqual(deployed, True)

        # Try to unload artifact with version 0.1.0
        unload_config_dict = generate_config(
            self.network, instances=instances, artifact_action="unload", artifact_version="0.1.0"
        )
        unload_config = Configuration(unload_config_dict)

        with Launcher(unload_config) as launcher:
            launcher.unload_all()
            launcher.wait_for_unload()

            self.wait_for_api_restart()
            explorer = launcher.explorer()

            for artifact in unload_config.artifacts.values():
                deployed = explorer.is_deployed(artifact)
                self.assertEqual(deployed, True)  # Not False !!!

            status, message = launcher.launch_state.unload_status
            self.assertEqual(status, ActionResult.Fail)
            self.assertTrue("service `101:cryptocurrency` references it as the current artifact" in message)

    def _tear_down(self, check_exit_codes=True):
        """Performs cleanup, removing network files."""

        if self.network is not None:
            outputs = self.network.stop()
            self.network.deinitialize()
            self.network = None

            if check_exit_codes:
                assert_processes_exited_successfully(self, outputs)

    def tearDown(self):
        self._tear_down()
