from flux.simulation.cell import ElectrolyzerCell
from flux.simulation.sensors import Sensor
from flux.pipeline.kafka_producer import SensorDataProducer
import time

if __name__ == "__main__":
    cell = ElectrolyzerCell("CELL_001")
    voltage_sensor = Sensor("V_001", "voltage", noise_stddev=0.005)

    # Create Kafka producer
    producer = SensorDataProducer()

    try:
        for i in range(5):
            cell_data = cell.step(dt=1.0)
            voltage_reading = voltage_sensor.read(cell_data["voltage"])

            # Prepare message
            message = {
                "cell_id": cell_data["cell_id"],
                "timestamp": cell_data["timestamp"],
                **voltage_reading,
            }

            # Send to Kafka
            producer.send("cell_voltage", message, key=cell_data["cell_id"])
            print(f"Sent: {message['sensor_id']} = {message['value']:.3f}V")

            time.sleep(1)

    finally:
        producer.flush()
        producer.close()
        print("Done")
