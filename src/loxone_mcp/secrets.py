"""Secure credential management using system keychain."""

import keyring
import os
import sys
import getpass
import socket
import asyncio
import json
import struct
from typing import Optional, List, Dict
import httpx
import logging

logger = logging.getLogger(__name__)


class LoxoneSecrets:
    """Manages Loxone credentials using the system keychain."""
    
    SERVICE_NAME = "LoxoneMCP"
    
    # Credential keys
    HOST_KEY = "LOXONE_HOST"
    USER_KEY = "LOXONE_USER"
    PASS_KEY = "LOXONE_PASS"
    
    @classmethod
    def get(cls, key: str) -> Optional[str]:
        """
        Retrieve a secret from environment variables or system keychain.
        
        Environment variables take precedence for CI/CD compatibility.
        
        Args:
            key: The credential key to retrieve
            
        Returns:
            The credential value or None if not found
        """
        # First check environment variables
        value = os.getenv(key)
        if value:
            return value
            
        # Then check system keychain
        try:
            return keyring.get_password(cls.SERVICE_NAME, key)
        except Exception as e:
            print(f"Warning: Could not access keychain: {e}", file=sys.stderr)
            return None
    
    @classmethod
    def set(cls, key: str, value: str) -> None:
        """Store a secret in the system keychain."""
        try:
            keyring.set_password(cls.SERVICE_NAME, key, value)
        except Exception as e:
            print(f"Error: Could not store credential in keychain: {e}", file=sys.stderr)
            raise
    
    @classmethod
    def delete(cls, key: str) -> None:
        """Remove a secret from the system keychain."""
        try:
            keyring.delete_password(cls.SERVICE_NAME, key)
        except keyring.errors.PasswordDeleteError:
            pass  # Already deleted
        except Exception as e:
            print(f"Warning: Could not delete credential: {e}", file=sys.stderr)
    
    @classmethod
    async def discover_loxone_servers(cls, timeout: float = 3.0) -> List[Dict[str, str]]:
        """Discover Loxone Miniservers on the local network."""
        servers = []
        
        # Method 1: Try common HTTP ports
        print("🔍 Scanning network for Loxone Miniservers...")
        
        # Get local network range
        try:
            # Get local IP
            s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            s.connect(("8.8.8.8", 80))
            local_ip = s.getsockname()[0]
            s.close()
            
            # Extract network prefix (e.g., 192.168.1.x -> 192.168.1)
            ip_parts = local_ip.split('.')
            network_prefix = '.'.join(ip_parts[:3])
            
            # Scan common IP ranges
            async def check_ip(ip: str) -> Optional[Dict[str, str]]:
                try:
                    # Try to connect to Loxone web interface
                    url = f"http://{ip}/data/LoxAPP3.json"
                    async with httpx.AsyncClient(timeout=1.0) as client:
                        # First check if port 80 is open
                        response = await client.get(f"http://{ip}/", follow_redirects=False)
                        
                        # Check if it's likely a Loxone (returns 401 for unauthorized)
                        if response.status_code in [401, 200]:
                            # Try to get more info
                            try:
                                # Some Loxone servers have a config endpoint
                                config_response = await client.get(f"http://{ip}/jdev/cfg/api", auth=("", ""))
                                if config_response.status_code == 200:
                                    data = config_response.json()
                                    name = data.get('LL', {}).get('value', {}).get('name', 'Loxone Miniserver')
                                else:
                                    name = "Loxone Miniserver"
                            except:
                                name = "Loxone Miniserver"
                            
                            return {
                                "ip": ip,
                                "name": name,
                                "port": "80"
                            }
                except:
                    pass
                return None
            
            # Scan network (common IPs)
            tasks = []
            for i in range(1, 255):
                ip = f"{network_prefix}.{i}"
                tasks.append(check_ip(ip))
            
            # Wait for results
            results = await asyncio.gather(*tasks)
            servers = [s for s in results if s is not None]
            
        except Exception as e:
            logger.debug(f"Network discovery error: {e}")
        
        return servers
    
    @classmethod
    async def _test_connection(cls, host: str, username: str, password: str) -> Dict[str, any]:
        """Test connection to Loxone Miniserver."""
        try:
            async with httpx.AsyncClient(timeout=5.0) as client:
                # Try to get structure file with credentials
                response = await client.get(
                    f"http://{host}/data/LoxAPP3.json",
                    auth=(username, password)
                )
                
                if response.status_code == 200:
                    # Success! Try to get some info
                    try:
                        data = response.json()
                        info = {
                            "name": data.get("msInfo", {}).get("projectName", "Unknown"),
                            "version": data.get("msInfo", {}).get("swVersion", "Unknown")
                        }
                        return {"success": True, "info": info}
                    except:
                        return {"success": True}
                elif response.status_code == 401:
                    return {"success": False, "error": "Invalid username or password"}
                else:
                    return {"success": False, "error": f"HTTP {response.status_code}"}
        except httpx.ConnectError:
            return {"success": False, "error": "Cannot connect to Miniserver"}
        except httpx.TimeoutException:
            return {"success": False, "error": "Connection timeout"}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    @classmethod
    def setup(cls) -> None:
        """Interactive setup wizard for configuring Loxone credentials."""
        print("🔐 Loxone MCP Server Setup")
        print("=" * 40)
        
        # Try to discover Loxone servers first
        discovered_servers = asyncio.run(cls.discover_loxone_servers())
        
        host = None
        if discovered_servers:
            print(f"\n✅ Found {len(discovered_servers)} Loxone Miniserver(s) on your network:\n")
            for i, server in enumerate(discovered_servers, 1):
                print(f"  {i}. {server['name']} at {server['ip']}")
            
            print("\nWould you like to use one of these servers?")
            choice = input("Enter the number (or press Enter to enter manually): ").strip()
            
            if choice.isdigit() and 1 <= int(choice) <= len(discovered_servers):
                selected = discovered_servers[int(choice) - 1]
                host = selected['ip']
                print(f"\nSelected: {selected['name']} at {host}")
        else:
            print("\nNo Loxone Miniservers found on the network.")
            print("You'll need to enter the IP address manually.")
        
        print("\nThis wizard will securely store your Loxone credentials")
        print("in your system keychain.\n")
        
        # Check for existing credentials
        existing = cls.get(cls.HOST_KEY) is not None
        if existing:
            response = input("Credentials already exist. Replace them? [y/N]: ")
            if response.lower() != 'y':
                print("Setup cancelled.")
                return
            print()
        
        # Collect credentials
        print("Please enter your Loxone Miniserver details:\n")
        
        # If no host was selected from discovery, ask for it
        if not host:
            host = input("Miniserver IP address (e.g., 192.168.1.100): ").strip()
            if not host:
                print("Error: Host cannot be empty")
                sys.exit(1)
            
        username = input("Username: ").strip()
        if not username:
            print("Error: Username cannot be empty")
            sys.exit(1)
            
        password = getpass.getpass("Password: ")
        if not password:
            print("Error: Password cannot be empty")
            sys.exit(1)
        
        # Test connection before saving
        print("\n🔌 Testing connection...")
        test_result = asyncio.run(cls._test_connection(host, username, password))
        
        if not test_result['success']:
            print(f"\n❌ Connection failed: {test_result['error']}")
            retry = input("\nWould you like to try again? [Y/n]: ")
            if retry.lower() != 'n':
                cls.setup()  # Restart setup
                return
            else:
                sys.exit(1)
        
        print(f"\n✅ Successfully connected to Loxone Miniserver!")
        if test_result.get('info'):
            print(f"   Miniserver: {test_result['info'].get('name', 'Unknown')}")
            print(f"   Version: {test_result['info'].get('version', 'Unknown')}")
        
        # Store credentials
        try:
            cls.set(cls.HOST_KEY, host)
            cls.set(cls.USER_KEY, username)
            cls.set(cls.PASS_KEY, password)
            
            print("\n✅ Credentials stored successfully!")
            print(f"   Host: {host}")
            print(f"   User: {username}")
            print(f"   Pass: {'*' * len(password)}")
            
            print("\n📝 Next steps:")
            print("1. Test the server: uv run mcp dev src/loxone_mcp/server.py")
            print("2. Configure in Claude Desktop (see README.md)")
            
        except Exception as e:
            print(f"\n❌ Error storing credentials: {e}")
            sys.exit(1)
    
    @classmethod
    def clear_all(cls) -> None:
        """Remove all stored credentials."""
        for key in [cls.HOST_KEY, cls.USER_KEY, cls.PASS_KEY]:
            cls.delete(key)
        print("✅ All credentials cleared")
    
    @classmethod
    def validate(cls) -> bool:
        """Check if all required credentials are available."""
        required = [cls.HOST_KEY, cls.USER_KEY, cls.PASS_KEY]
        missing = [key for key in required if not cls.get(key)]
        
        if missing:
            print(f"❌ Missing credentials: {', '.join(missing)}")
            print("Run 'uvx --from . loxone-mcp setup' to configure")
            return False
            
        return True


if __name__ == "__main__":
    # Allow running this file directly for setup
    if len(sys.argv) > 1:
        if sys.argv[1] == "setup":
            LoxoneSecrets.setup()
        elif sys.argv[1] == "clear":
            LoxoneSecrets.clear_all()
        else:
            print(f"Unknown command: {sys.argv[1]}")
            print("Usage: python secrets.py [setup|clear]")
    else:
        # Validate existing credentials
        if LoxoneSecrets.validate():
            print("✅ All credentials are configured")
